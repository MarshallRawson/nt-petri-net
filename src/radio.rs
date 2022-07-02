use mnet_lib::{Place, GraphMaker, GraphRunner};
use mnet_macro::MnetPlace;
use std::{thread, time};

#[derive(MnetPlace)]
#[mnet_place(f, (), ())]
struct HackOneRf;
impl HackOneRf {
    // get samples from hack one rf (complex)
    fn f(&mut self, _: ()) -> () {
        thread::sleep(time::Duration::from_millis(10));
    }
}

#[derive(MnetPlace)]
#[mnet_place(f, (), ())]
struct FreqShift;
impl FreqShift {
    // multiply samples by sin (frequency shift
    fn f(&mut self, _: ()) -> () {
    }
}

#[derive(MnetPlace)]
#[mnet_place(f, (), ())]
struct LowPass;
impl LowPass {
    // Low pass filter
    fn f(&mut self, _: ()) -> () {
    }
}

#[derive(MnetPlace)]
#[mnet_place(f, (), ())]
struct Plot;
impl Plot {
    // plot waterfall, fft, and time series data
    fn f(&mut self, _: ()) -> () {
    }
}


fn main() {
    let g = GraphMaker::make()
        .set_start_tokens::<()>("start", vec![()])
        .edge_to_place("start", "HackOneRf").add_place("HackOneRf", Box::new(HackOneRf{}))
        .place_to_edge("HackOneRf", "raw").add_edge::<()>("raw")
        .edge_to_place("raw", "FreqShift").add_place("FreqShift", Box::new(FreqShift{}))
        .place_to_edge("FreqShift", "centered").add_edge::<()>("centered")
        .edge_to_place("centered", "LowPass").add_place("LowPass", Box::new(LowPass{}))
        .place_to_edge("LowPass", "filtered").add_edge::<()>("filtered")
        .edge_to_place("filtered", "Plot").add_place("Plot", Box::new(Plot{}))
        .place_to_edge("Plot", "start")
    ;
    println!("{:?}", GraphRunner::from_maker(g).run());
}
