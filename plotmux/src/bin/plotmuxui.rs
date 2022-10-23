use plotmux::plotmuxui::PlotMuxUi;

use clap::Parser;
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    graph_png: Option<String>,
    #[arg(long)]
    port: u32,
    #[arg(long)]
    sources: Vec<String>,
}

fn main() {
    let args = Args::parse();
    println!("using png: {:?} and port: {}", args.graph_png, args.port);
    PlotMuxUi::make(args.graph_png.as_ref(), args.port, args.sources).spin();
}
