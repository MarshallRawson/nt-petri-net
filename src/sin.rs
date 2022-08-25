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
//use ntpnet_macro;
use ntpnet_lib;

mod sin {
    #[derive(ntpnet_macro::Fire)]
    struct X { x: f64 }
    #[derive(ntpnet_macro::Product)]
    struct Success { sin: f64 }
    #[derive(ntpnet_macro::Product)]
    struct Nan { nan: f64 }
    #[derive(ntpnet_macro::Transition)]
    #[ntpnet_in_edges(x,)]
    #[ntpnet_out_edges(sin, nan)]
    //#[ntpnet_transitions(f: Fire(X) -> Product(Success, Nan),)]
    struct Sin {
        //p: PlotSink,
    }
    //impl Sin {
    //    fn f(&mut self, x: Fire) -> Product {
    //        // extract value(s) from the fire conditions
    //        let x = match x { Fire::X(x) => { x.x } };
    //        let sin = x.sin();
    //        // pack value(s) back into one of a given product
    //        match sin {
    //            std::f64::NAN => Product::Nan(Nan{nan: std::f64::NAN}),
    //            _ => Product::Success(Success{sin}),
    //        }
    //    }
    //}
}

fn main() {
    println!("hello world!");
}

