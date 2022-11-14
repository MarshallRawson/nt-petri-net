use std::time::Instant;

use ntpnet_macro;
use ntpnet_lib;
use plotmux::plotsink::PlotSink;

use crate::facial_recognition::Face;

#[derive(ntpnet_macro::TransitionInputTokens)]
struct VoiceFace {
    audio: (Instant, Vec<i16>),
    faces: (Instant, Vec<Face>),
}

#[derive(ntpnet_macro::TransitionOutputTokens)]
struct VoiceFaceEnable {
    audio_enable: (),
    face_enable: (),
}

#[derive(ntpnet_macro::Transition)]
#[ntpnet_transition(f: Input(VoiceFace) -> Output(VoiceFaceEnable))]
pub struct VoiceFaceSync {
    p: PlotSink,
}
impl VoiceFaceSync {
    pub fn maker(p: PlotSink) -> ntpnet_lib::TransitionMaker {
        Box::new(|| {
            Box::new(VoiceFaceSync{
                p
            })
        })
    }
    fn f(&mut self, i: Input) -> Output {
        Output::VoiceFaceEnable(VoiceFaceEnable{ audio_enable: (), face_enable: () })
    }
}
