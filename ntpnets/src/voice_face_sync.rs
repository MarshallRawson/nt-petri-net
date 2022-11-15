use std::time::Instant;
use std::collections::VecDeque;

use ntpnet_macro;
use ntpnet_lib;
use plotmux::plotsink::PlotSink;

use crate::facial_recognition::Face;

#[derive(ntpnet_macro::TransitionInputTokens)]
struct Faces {
    faces: (Instant, Vec<Face>),
}
#[derive(ntpnet_macro::TransitionOutputTokens)]
struct FacesEnable {
    faces_enable: (),
}

#[derive(ntpnet_macro::TransitionInputTokens)]
struct Voice {
    audio: (Instant, Vec<i16>),
}
#[derive(ntpnet_macro::TransitionOutputTokens)]
struct VoiceEnable {
    audio_enable: (),
}


#[derive(ntpnet_macro::Transition)]
#[ntpnet_transition(video: VideoInput(Faces) -> VideoOutput(FacesEnable))]
#[ntpnet_transition(audio: AudioInput(Voice) -> AudioOutput(VoiceEnable))]
pub struct VoiceFaceSync {
    p: PlotSink,
    start: Instant,
    audio: VecDeque<f32>,
    tl: Option<Instant>,
}
impl VoiceFaceSync {
    pub fn maker(p: PlotSink) -> ntpnet_lib::TransitionMaker {
        Box::new(|| {
            Box::new(VoiceFaceSync{
                p: p,
                start: Instant::now(),
                audio: VecDeque::new(),
                tl: None,
            })
        })
    }
    fn audio(&mut self, i: AudioInput) -> AudioOutput {
        let (t, samples) = match i { AudioInput::Voice(Voice { audio: (t, samples) }) => (t, samples) };
        self.process_audio(t, samples);
        AudioOutput::VoiceEnable(VoiceEnable{ audio_enable: () })
    }
    fn process_audio(&mut self, t: Instant, samples: Vec<i16>) {
        if let Some(tl) = self.tl {
            self.p.plot_series_2d("", "audio timing residual",
                (t - self.start).as_secs_f64(), (t - tl).as_secs_f64() - samples.len() as f64 / 44100.
            );
        }
        for s in samples {
            self.audio.push_back(s as f32);
            if self.audio.len() > 4410 {
                self.audio.pop_front();
            }
        }
        self.p.plot_line_2d("", "", self.audio.iter().enumerate().map(|(x, y)| (x as f64, *y as f64)).collect());
        self.tl = Some(t);
    }
    fn video(&mut self, _i: VideoInput) -> VideoOutput {
        //let (t, samples) = match i { Input::VoiceFace(VoiceFace { audio: (t, samples), faces: _}) => (t, samples) };
        //self.process_audio(t, samples);
        VideoOutput::FacesEnable(FacesEnable{ faces_enable: () })
    }
}
