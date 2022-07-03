use mnet_lib::{Place, PlaceMaker, graph};
use mnet_macro::MnetPlace;
use std::{thread, time};

#[derive(MnetPlace)]
#[mnet_place(f, (), ())]
struct HackOneRf;
impl HackOneRf {
    fn maker() -> PlaceMaker {
        PlaceMaker!(Box::new(move || Box::new(Self{})))
    }
    // get samples from hack one rf (complex)
    fn f(&mut self, _: ()) -> () {
        thread::sleep(time::Duration::from_millis(10));
    }
}

#[derive(MnetPlace)]
#[mnet_place(f, (), ())]
struct FreqShift;
impl FreqShift {
    fn maker() -> PlaceMaker {
        PlaceMaker!(Box::new(move || Box::new(Self{})))
    }
    // multiply samples by sin (frequency shift
    fn f(&mut self, _: ()) -> () {
    }
}

#[derive(MnetPlace)]
#[mnet_place(f, (), ())]
struct LowPass;
impl LowPass {
    fn maker() -> PlaceMaker {
        PlaceMaker!(Box::new(move || Box::new(Self{})))
    }
    // Low pass filter
    fn f(&mut self, _: ()) -> () {
    }
}

#[derive(MnetPlace)]
#[mnet_place(f, (), ())]
struct Plot;
impl Plot {
    fn maker() -> PlaceMaker {
        PlaceMaker!(Box::new(move || Box::new(Self{})))
    }
    // plot waterfall, fft, and time series data
    fn f(&mut self, _: ()) -> () {
    }
}

fn main() {
    let g = graph::Maker::make()
        .set_start_tokens::<()>("start", vec![()])
        .edge_to_place("start", "HackOneRf").add_place("HackOneRf", HackOneRf::maker())
        .place_to_edge("HackOneRf", "raw").add_edge::<()>("raw")
        .edge_to_place("raw", "FreqShift").add_place("FreqShift", FreqShift::maker())
        .place_to_edge("FreqShift", "centered").add_edge::<()>("centered")
        .edge_to_place("centered", "LowPass").add_place("LowPass", LowPass::maker())
        .place_to_edge("LowPass", "filtered").add_edge::<()>("filtered")
        .edge_to_place("filtered", "Plot").add_place("Plot", Plot::maker())
        .place_to_edge("Plot", "start")
    ;
    println!("{:?}", graph::Runner::from_maker(g).run());
}
