use plotmux::plotmuxui::PlotMuxUi;
use std::env;
use std::collections::VecDeque;

fn main() {
    let mut args : VecDeque<String> = env::args().collect::<VecDeque<String>>();
    args.pop_front();
    let port = args[0].parse().expect("PORT arg malformed!");
    args.pop_front();
    PlotMuxUi::make(port, args.into()).spin();
}
