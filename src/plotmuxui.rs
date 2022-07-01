use plotmux::plotmuxui::PlotMuxUi;
use std::env;
use std::collections::VecDeque;

fn main() {
    let mut args : VecDeque<String> = env::args().collect::<VecDeque<String>>();
    args.pop_front();
    let graph_png_path = args[0].parse::<String>().expect("GRAPH_PNG arg malformed!");
    args.pop_front();
    let port = args[0].parse().expect("PORT arg malformed!");
    args.pop_front();
    PlotMuxUi::make(&graph_png_path, port, args.into()).spin();
}
