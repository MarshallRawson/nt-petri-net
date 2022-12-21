use crate::{net::Net, multi_reactor::MultiReactor};
use plotmux::plotmux::PlotMux;

pub fn reactor(n: Net, plotmux: &mut PlotMux) -> MultiReactor {
    let wc = vec![n.transitions.keys().cloned().collect()];
    MultiReactor::make(n, wc, plotmux)
}






