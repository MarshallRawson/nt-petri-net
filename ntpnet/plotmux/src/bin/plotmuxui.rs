use plotmux::plotmuxui::PlotMuxUi;

use clap::Parser;
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    graph_png: Option<String>,
    #[arg(long)]
    addr: String,
}

fn main() {
    let args = Args::parse();
    println!("using png: {:?} and addr: {}", args.graph_png, args.addr);
    PlotMuxUi::make(args.graph_png.as_ref(), args.addr).spin();
}
