mod plotmux;
use plotters::prelude::{BitMapBackend};


struct PlotSource {
}

struct PlotSink {

}

trait PlotableData {
    fn plot(&self, &mut area: BitMapBackend);
}


struct PlotMux {
    a:
}









