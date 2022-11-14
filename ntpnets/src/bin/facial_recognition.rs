use ntpnets::camera_reader::CameraReader;
use ntpnets::facial_recognition::FacialRecognition;

use ntpnet_lib::{net::Net, reactor::Reactor};
use plotmux::plotmux::{PlotMux, ClientMode};

use clap::Parser;
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = 30)]
    fps: u32,
    #[arg(short, long)]
    remote_plotmux: Option<String>,
}

fn main() {
    let args = Args::parse();

    let mut plotmux = PlotMux::make();
    let n = Net::make()
        .set_start_tokens("E", vec![Box::new(())])
        .place_to_transition("E", "_enable", "camera_reader")
        .add_transition(
            "camera_reader",
            CameraReader::maker(
                args.fps,
                plotmux.add_plot_sink("camera_reader"),
            ),
        )
        .add_transition(
            "facial_recognition",
            FacialRecognition::maker(plotmux.add_plot_sink("facial_recognition")),
        )
        .transition_to_place("camera_reader", "image", "Image")
        .place_to_transition("Image", "image", "facial_recognition")
        .transition_to_place("facial_recognition", "out", "E");
    let png = n.png();
    let r = Reactor::make(n, &mut plotmux);
    let plotmux_mode = if let Some(addr) = args.remote_plotmux {
        ClientMode::Remote(addr)
    } else {
        ClientMode::Local()
    };
    plotmux.make_ready(Some(&png), plotmux_mode);
    r.run();
}
