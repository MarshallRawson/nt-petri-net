//use mnet_lib::{graph, Place, PlaceMaker};
//use mnet_macro::MnetPlace;
//use plotmux::{plotmux::PlotMux, plotsink::PlotSink};
//use std::{thread, time};
//
//#[derive(MnetPlace)]
//#[mnet_place(f, f64, f64)]
//struct Sin {
//    p: PlotSink,
//}
//impl Sin {
//    fn f(&mut self, t: f64) -> f64 {
//        self.p.plot_series_2d("sin(t)".into(), t, t.sin());
//        thread::sleep(time::Duration::from_millis(10));
//        t + 0.01
//    }
//    fn maker(plotsink: PlotSink) -> PlaceMaker {
//        PlaceMaker!(Box::new(move || Box::new(Sin { p: plotsink })))
//    }
//}
//
//fn main() {
//    let mut plotmux = PlotMux::make();
//    let g = graph::Maker::make()
//        .set_start_tokens::<f64>("time", vec![0.])
//        .edge_to_place("time", "sin")
//        .add_place("sin", Sin::maker(plotmux.add_plot_sink("sin")))
//        .place_to_edge("sin", "time");
//    plotmux.make_ready(&g.png());
//    thread::spawn(move || plotmux.spin());
//    println!("{:?}", graph::Runner::from_maker(g).run());
//}

///////////////////////////////

//use ntpnet_lib::{ntpnet, Transition};
use ntpnet_macro;
use ntpnet_lib;

mod sin {
    #[derive(ntpnet_macro::Fire)]
    #[derive(ntpnet_macro::Product)]
    struct Rad { rad: f64 }
    #[derive(ntpnet_macro::Fire)]
    #[derive(ntpnet_macro::Product)]
    struct Deg { deg: f64 }
    #[derive(ntpnet_macro::Transition)]
    #[ntpnet_transition(rad: FireRad(Rad) -> ProductRad(Rad))]
    #[ntpnet_transition(deg: FireDeg(Deg) -> ProductDeg(Deg))]
    pub struct Sin {
        //p: PlotSink,
    }
    impl Sin {
        fn rad(&mut self, rad: FireRad) -> ProductRad {
            match rad {
                FireRad::Rad(Rad { rad }) => ProductRad::Rad(Rad { rad: rad.sin() }),
            }
        }
        fn deg(&mut self, deg: FireDeg) -> ProductDeg {
            match deg {
                FireDeg::Deg(Deg { deg }) => ProductDeg::Deg(Deg { deg: deg.sin() }),
            }
        }
    }
}

fn main() {
    println!("hello world!");
    sin::Sin{};
}

