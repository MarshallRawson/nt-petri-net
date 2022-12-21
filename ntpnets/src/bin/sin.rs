mod sin {
    use ntpnet_lib::TransitionMaker;
    use plotmux::plotsink::PlotSink;
    use std::{thread, time};
    #[derive(ntpnet_macro::TransitionInputTokens, ntpnet_macro::TransitionOutputTokens)]
    struct Time {
        t: f64,
    }
    #[derive(ntpnet_macro::Transition)]
    #[ntpnet_transition(sin: Input(Time) -> Output(Time))]
    pub struct Sin {
        p: PlotSink,
    }
    impl Sin {
        pub fn maker(plotsink: PlotSink) -> TransitionMaker {
            Box::new(move || Box::new(Sin { p: plotsink }))
        }
        fn sin(&mut self, f: Input) -> Output {
            let t = match f {
                Input::Time(Time { t }) => t,
            };
            self.p.plot_series_2d("", "sin(t)".into(), t, t.sin());
            self.p.println2("t", &format!("{}", t));
            self.p.println2("sin(t)", &format!("{}", t.sin()));
            thread::sleep(time::Duration::from_millis(10));
            Output::Time(Time { t: t + 0.01 })
        }
    }
}

use ntpnet_lib::{net::Net, reactor::reactor, Token};
use plotmux::plotmux::{ClientMode, PlotMux};

fn main() {
    let mut plotmux = PlotMux::make();
    let n = Net::make()
        .set_start_tokens("time", vec![Token::new(0.)])
        .place_to_transition("time", "t", "sin")
        .add_transition("sin", sin::Sin::maker(plotmux.add_plot_sink("sin")))
        .transition_to_place("sin", "t", "time");
    let png = n.png();
    let r = reactor(n, &mut plotmux);
    let pm = plotmux.make_ready(Some(&png), ClientMode::Local());
    r.run(&None);
    drop(pm);
}
