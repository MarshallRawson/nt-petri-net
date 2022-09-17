mod sin {
    use plotmux::plotsink::PlotSink;
    use std::{thread, time};
    use ntpnet_lib::TransitionMaker;
    #[derive(ntpnet_macro::Fire)]
    #[derive(ntpnet_macro::Product)]
    pub struct Time { pub t: f64 }
    #[derive(ntpnet_macro::Transition)]
    #[ntpnet_transition(sin: Fire(Time) -> Product(Time))]
    pub struct Sin {
        p: PlotSink,
    }
    impl Sin {
        pub fn maker(plotsink: PlotSink) -> TransitionMaker {
            Box::new(move || Box::new(Sin { p: plotsink }))
        }
        fn sin(&mut self, f: Fire) -> Product {
            let t = match f {
                Fire::Time(Time { t }) => t
            };
            self.p.plot_series_2d("sin(t)".into(), t, t.sin());
            thread::sleep(time::Duration::from_millis(10));

            Product::Time(Time { t: t + 0.01 })
        }
    }
}

use ntpnet_lib::{net::Net, reactor::Reactor, Token};
use plotmux::plotmux::PlotMux;
use std::thread;
use std::any::{Any, TypeId};

fn main() {
    let mut plotmux = PlotMux::make();
    println!("TypeId::of::<f64>(): {:?}", TypeId::of::<f64>());
    let n = Net::make()
        .set_start_tokens("time", vec![Box::new(0.)])
        .place_to_transition("time", "t", "sin")
        .add_transition("sin", sin::Sin::maker(plotmux.add_plot_sink("sin")))
        .transition_to_place("sin", "t", "time")
    ;
    //plotmux.make_ready(&n.png());
    //thread::spawn(move || plotmux.spin());
    Reactor::make(n).run();
}

