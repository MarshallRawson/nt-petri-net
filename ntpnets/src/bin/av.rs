#![feature(trait_upcasting)]
#![allow(incomplete_features)]

use std::collections::HashSet;

use ntpnet_lib::{multi_reactor::{MultiReactor, PlotOptions}, net::Net};
use ntpnets::camera_reader::CameraReader;
use ntpnets::facial_recognition::FacialRecognition;
use ntpnets::sound_reader::SoundReader;
use ntpnets::voice_face_sync::VoiceFaceSync;
use plotmux::plotmux::{ClientMode, PlotMux};

fn main() {
    let mut plotmux = PlotMux::make();
    let n = Net::make()
        .set_start_tokens("sound_enable", vec![Box::new(())])
        .set_start_tokens("face_enable", vec![Box::new(())])
        .set_start_tokens("camera_enable", vec![Box::new(())])
        .place_to_transition("sound_enable", "_e", "sound_reader")
        .add_transition(
            "sound_reader",
            SoundReader::maker(plotmux.add_plot_sink("sound_reader")),
        )
        .transition_to_place("sound_reader", "samples", "sound_samples")
        .place_to_transition("sound_samples", "audio", "sync")
        .transition_to_place("sync", "audio_enable", "sound_enable")
        .transition_to_place("sync", "faces_enable", "face_enable")
        .add_transition("sync", VoiceFaceSync::maker(30, plotmux.add_plot_sink("sync")))
        .place_to_transition("face_enable", "_e", "facial_recognition")
        .add_transition(
            "facial_recognition",
            FacialRecognition::maker(plotmux.add_plot_sink("facial_recognition")),
        )
        .transition_to_place("facial_recognition", "faces", "faces")
        .place_to_transition("faces", "faces", "sync")
        .transition_to_place("facial_recognition", "next_image", "camera_enable")
        .transition_to_place("camera_reader", "image", "image")
        .place_to_transition("image", "image", "facial_recognition")
        .add_transition(
            "camera_reader",
            CameraReader::maker(30, plotmux.add_plot_sink("camera_reader")),
        )
        .place_to_transition("camera_enable", "_enable", "camera_reader");
    let r = MultiReactor::make(
        n,
        vec![
            HashSet::from(["sound_reader".into()]),
            HashSet::from(["camera_reader".into()]),
            HashSet::from(["facial_recognition".into()]),
            HashSet::from(["sync".into()]),
        ],
        &mut plotmux,
    );
    plotmux.make_ready(Some(&r.png()), ClientMode::Local());
    r.run(PlotOptions{
        reactor_timing: true,
        transition_timing: true,
        state: true,
    });
}
