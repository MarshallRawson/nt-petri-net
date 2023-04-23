use crate::{multi_reactor::MultiReactor, net::Net};
use plotmux::plotmux::PlotMux;

pub fn reactor(n: Net, plotmux: &mut PlotMux) -> MultiReactor {
    let wc = vec![n.transitions.keys().cloned().collect()];
    MultiReactor::make(n, wc, plotmux)
}
