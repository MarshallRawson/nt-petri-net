use utilities::{
    audio_fft::AudioFFT, audio_mouth_moving::AudioMouthMoving, camera_reader::CameraReader,
    facial_detection::FacialDetection, mouth_moving::MouthMoving, sound_reader::SoundReader,
};

use ntpnet::{MultiReactor, Net, ReactorOptions, Token};
use plotmux::plotmux::{ClientMode, PlotMux};
use std::collections::HashSet;

use clap::Parser;
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = 30)]
    fps: u32,
    #[arg(short, long, default_value_t = String::from("/dev/video0"))]
    dev: String,
    #[arg(short, long)]
    remote_plotmux: Option<String>,
    #[command(subcommand)]
    reactor_plot_options: Option<ReactorOptions>,
}

fn main() {
    let args = Args::parse();

    let plotmux_mode = ClientMode::parse(args.remote_plotmux);
    let mut plotmux = PlotMux::make(plotmux_mode);
    let n = Net::make()
        .set_start_tokens("camera_enable", vec![Token::new(())])
        .add_transition(
            "camera_reader",
            CameraReader::maker(args.fps, args.dev, plotmux.add_plot_sink("camera_reader")),
        )
        .add_transition(
            "face_detection",
            FacialDetection::maker(plotmux.add_plot_sink("face_detection")),
        )
        .add_transition(
            "mouth_moving",
            MouthMoving::maker(plotmux.add_plot_sink("mouth_moving")),
        )
        .place_to_transition("camera_enable", "_enable", "camera_reader")
        .transition_to_place("camera_reader", "image", "Image")
        .place_to_transition("Image", "image", "face_detection")
        .place_to_transition("face_detection_enable", "_enable", "face_detection")
        .set_start_tokens("face_detection_enable", vec![Token::new(())])
        .transition_to_place("face_detection", "done", "camera_enable")
        .transition_to_place("face_detection", "faces", "Faces")
        .place_to_transition("Faces", "faces", "mouth_moving")
        .transition_to_place("mouth_moving", "done", "face_detection_enable")
        .transition_to_place("mouth_moving", "moving", "MouthMoving")
        .place_to_transition("mouth_moving_enable", "_enable", "mouth_moving")
        .set_start_tokens("mouth_moving_enable", vec![Token::new(())])
        .set_start_tokens("sound_reader_enable", vec![Token::new(())])
        .place_to_transition("sound_reader_enable", "_e", "sound_reader")
        .add_transition(
            "sound_reader",
            SoundReader::maker(plotmux.add_plot_sink("sound_reader")),
        )
        .add_transition(
            "audio_fft",
            AudioFFT::maker(plotmux.add_plot_sink("audio_fft")),
        )
        .transition_to_place("audio_fft", "done", "sound_reader_enable")
        .set_start_tokens("fft_enable", vec![Token::new(())])
        .place_to_transition("fft_enable", "_enable", "audio_fft")
        .transition_to_place("audio_fft", "fft", "FFT")
        .place_to_transition("audio", "audio", "audio_fft")
        .transition_to_place("sound_reader", "samples", "audio")
        .add_transition(
            "audio_mouth_moving",
            AudioMouthMoving::maker(plotmux.add_plot_sink("audio_mouth_moving")),
        )
        .place_to_transition("FFT", "audio", "audio_mouth_moving")
        .place_to_transition("MouthMoving", "mouth_moving", "audio_mouth_moving")
        .transition_to_place("audio_mouth_moving", "audio_done", "fft_enable")
        .transition_to_place(
            "audio_mouth_moving",
            "mouth_moving_done",
            "mouth_moving_enable",
        );
    let r = MultiReactor::make(
        n,
        vec![
            HashSet::from(["camera_reader".into()]),
            HashSet::from(["face_detection".into()]),
            HashSet::from(["mouth_moving".into()]),
            HashSet::from(["sound_reader".into()]),
            HashSet::from(["audio_fft".into()]),
            HashSet::from(["audio_mouth_moving".into()]),
        ],
        &mut plotmux,
    );
    println!("{:?}", r.png());
    let pm = plotmux.make_ready(Some(&r.png()));
    r.run(&args.reactor_plot_options);
    drop(pm);
}
