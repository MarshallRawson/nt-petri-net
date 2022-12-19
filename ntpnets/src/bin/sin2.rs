#![feature(trait_upcasting)]
#![allow(incomplete_features)]

use clap::Parser;
use ntpnet_lib::{multi_reactor::MultiReactor, net::Net, ReactorOptions};
use plotmux::plotmux::{ClientMode, PlotMux};

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    reactor_plot_options: Option<ReactorOptions>,
}

mod sin_gen {
    use ntpnet_lib::TransitionMaker;
    use plotmux::plotsink::PlotSink;
    #[derive(ntpnet_macro::TransitionInputTokens)]
    struct TimeSpan {
        t: (f64, f64, usize),
    }
    #[derive(ntpnet_macro::TransitionOutputTokens)]
    struct F {
        f: Vec<f64>,
    }
    #[derive(ntpnet_macro::Transition)]
    #[ntpnet_transition(sin: Input(TimeSpan) -> Output(F))]
    pub struct SinGen {
        p: PlotSink,
    }
    impl SinGen {
        pub fn maker(plotsink: PlotSink) -> TransitionMaker {
            Box::new(move || Box::new(Self { p: plotsink }))
        }
        fn sin(&mut self, f: Input) -> Output {
            let (bot, top, n) = match f {
                Input::TimeSpan(TimeSpan { t: (b, t, n) }) => (b, t, n),
            };
            let t = (0..n).map(|i| bot + i as f64 * (top - bot) / n as f64).collect::<Vec<f64>>();
            let f = t.iter().map(|t| t.sin()).collect::<Vec<_>>();
            self.p.plot_series_2d_vec("", "sin", std::iter::zip(t, f.clone()).collect());
            Output::F(F { f: f })
        }
    }
}

mod fft_real {
    use rustfft::{num_complex::Complex, FftPlanner};
    use ntpnet_lib::TransitionMaker;
    use plotmux::plotsink::PlotSink;
    #[derive(ntpnet_macro::TransitionInputTokens)]
    struct Time {
        s: Vec<f64>,
    }
    #[derive(ntpnet_macro::TransitionOutputTokens)]
    struct Freq {
        s: Vec<Complex<f64>>,
    }
    #[derive(ntpnet_macro::Transition)]
    #[ntpnet_transition(fft: Input(Time) -> Output(Freq))]
    pub struct FFTReal {
        p: PlotSink,
        fft_planner: FftPlanner<f64>,
    }
    impl FFTReal {
        pub fn maker(plotsink: PlotSink) -> TransitionMaker {
            Box::new(move || Box::new(Self {
                p: plotsink,
                fft_planner: FftPlanner::new(),
            }))
        }
        fn fft(&mut self, s: Input) -> Output {
            let s = match s {
                Input::Time(Time { s }) => s,
            };
            let fft = self.fft_planner.plan_fft_forward(s.len());
            let mut s = s
                .iter()
                .map(|x| Complex {
                    re: *x,
                    im: 0.,
                })
                .collect::<Vec<_>>();
            fft.process(&mut s);
            self.p.plot_line_2d(
                "frequency",
                "|fft(s)|",
                s[0..s.len()/2]
                    .iter()
                    .enumerate()
                    .map(|(x, y)| {
                        (
                            x as f64,
                            y.norm(),
                        )
                    })
                    .collect(),
            );
            self.p.plot_line_2d(
                "frequency",
                "arg(fft(s))",
                s[0..s.len()/2]
                    .iter()
                    .enumerate()
                    .map(|(x, y)| {
                        (
                            x as f64,
                            y.arg(),
                        )
                    })
                    .collect(),
            );
            Output::Freq(Freq { s: s })
        }
    }
}


fn main() {
    let args = Args::parse();
    let mut plotmux = PlotMux::make();
    let n = Net::make()
        .set_start_tokens("time", vec![Box::new((0.0, std::f64::consts::PI * 2., 1000_usize))])
        .place_to_transition("time", "t", "sin_gen")
        .add_transition(
            "sin_gen",
            sin_gen::SinGen::maker(plotmux.add_plot_sink("sin_gen")),
        )
        .transition_to_place("sin_gen", "f", "s")
        .place_to_transition("s", "s", "fft")
        .add_transition(
            "fft",
            fft_real::FFTReal::maker(plotmux.add_plot_sink("fft")),
        )
        .transition_to_place("fft", "s", "S");
    let wc = vec![n.transitions.keys().cloned().collect()];
    let r = MultiReactor::make(n, wc, &mut plotmux);
    plotmux.make_ready(Some(&r.png()), ClientMode::Local());
    r.run(&args.reactor_plot_options);
}
