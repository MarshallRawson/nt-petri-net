use ntpnets::camera_reader::CameraReader;
use ntpnets::yolo::Yolo as ImageConsumer;

use ntpnet_lib::{net::Net, multi_reactor::MultiReactor};
use plotmux::plotmux::{PlotMux, ClientMode};

use std::collections::HashSet;
use image::ImageBuffer;
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
            "image_consumer",
            ImageConsumer::maker(plotmux.add_plot_sink("image_consumer")),
        )
        .set_start_tokens("Image", vec![Box::new(ImageBuffer::from_pixel(1, 1, image::Rgb([0_u8, 0, 0])))])
        .transition_to_place("camera_reader", "image", "Image")
        .place_to_transition("Image", "image", "image_consumer")
        .transition_to_place("image_consumer", "out", "E");
    let multi_reactor = MultiReactor::make(n,
        vec![HashSet::from(["camera_reader".into()]), HashSet::from(["image_consumer".into()])],
        &mut plotmux
    );
    let plotmux_mode = if let Some(addr) = args.remote_plotmux {
        ClientMode::Remote(addr)
    } else {
        ClientMode::Local()
    };
    plotmux.make_ready(Some(&multi_reactor.png()), plotmux_mode);
    multi_reactor.run();
}
