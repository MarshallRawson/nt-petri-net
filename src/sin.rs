mod sin {
    use ntpnet_lib::TransitionMaker;
    use plotmux::plotsink::PlotSink;
    use std::{thread, time};
    #[derive(ntpnet_macro::TransitionInputTokens)]
    #[derive(ntpnet_macro::TransitionOutputTokens)]
    pub struct Time {
        pub t: f64,
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
            self.p.plot_series_2d("sin(t)".into(), t, t.sin());
            thread::sleep(time::Duration::from_millis(10));

            Output::Time(Time { t: t + 0.01 })
        }
    }
}

use ntpnet_lib::{net::Net, reactor::Reactor, Token};
use plotmux::plotmux::PlotMux;
use std::any::{Any, TypeId};
use std::thread;

fn main() {
    let mut plotmux = PlotMux::make();
    let n = Net::make()
        .set_start_tokens("time", vec![Box::new(0.)])
        .place_to_transition("time", "t", "sin")
        .add_transition("sin", sin::Sin::maker(plotmux.add_plot_sink("sin")))
        .transition_to_place("sin", "t", "time");
    plotmux.make_ready(&n.png());
    thread::spawn(move || plotmux.spin());
    Reactor::make(n).run();
}
