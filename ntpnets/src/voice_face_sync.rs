use std::collections::VecDeque;
use std::time::{Duration, Instant};

use ntpnet_lib;
use ntpnet_macro;
use plotmux::plotsink::{ImageCompression, PlotSink};

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

enum PopRet {
    TooEarly,
    Some(Vec<f64>),
    TooLate,
}
struct TimeSeries {
    data: VecDeque<f64>,
    fs: f64,
    time_range: (Instant, Instant),
}
impl TimeSeries {
    fn new(end_time: Instant, fs: f64, data: Vec<f64>) -> Self {
        Self {
            time_range: (
                end_time - Duration::from_secs_f64(data.len() as f64 * fs),
                end_time,
            ),
            data: VecDeque::from_iter(data),
            fs: fs,
        }
    }
    fn extend(&mut self, end_time: Instant, data: Vec<f64>) {
        self.data.extend(data);
        self.time_range.1 = end_time;
    }
    fn pop(&mut self, end_time: Instant, duration: Duration) -> PopRet {
        if self.time_range.1 < end_time {
            PopRet::TooLate
        } else if end_time + duration < self.time_range.0 {
            PopRet::TooEarly
        } else {
            let requested_samples = (duration.as_secs_f64() * self.fs) as usize;
            let popped_samples = (((end_time - self.time_range.0).as_secs_f64() * self.fs) as usize)
                .clamp(0, self.data.len());
            self.time_range.0 = end_time;
            PopRet::Some(self.data
                .drain(0..popped_samples)
                .enumerate()
                .filter_map(|(i, x)| {
                    if popped_samples - i <= requested_samples {
                        Some(x)
                    } else {
                        None
                    }
                })
                .collect()
            )
        }
    }
}

#[derive(ntpnet_macro::Transition)]
#[ntpnet_transition(video: VideoInput(Faces) -> VideoOutput(FacesEnable))]
#[ntpnet_transition(audio: AudioInput(Voice) -> AudioOutput(VoiceEnable))]
pub struct VoiceFaceSync {
    fps: usize,
    p: PlotSink,
    start: Instant,
    last_audio: Option<Instant>,
    audio: Option<TimeSeries>,
    video: VecDeque<(Instant, Vec<Face>)>,
}
impl VoiceFaceSync {
    pub fn maker(fps: usize, p: PlotSink) -> ntpnet_lib::TransitionMaker {
        Box::new(move || {
            Box::new(VoiceFaceSync {
                fps: fps,
                p: p,
                start: Instant::now(),
                last_audio: None,
                audio: None,
                video: VecDeque::new(),
            })
        })
    }
    fn audio(&mut self, i: AudioInput) -> AudioOutput {
        let (t, samples) = match i {
            AudioInput::Voice(Voice {
                audio: (t, samples),
            }) => (t, samples),
        };
        if let Some(audio) = &mut self.audio {
            audio.extend(t, samples.iter().map(|x| *x as f64).collect());
            self.p.plot_line_2d(
                "audio",
                "audio",
                audio
                    .data
                    .iter()
                    .enumerate()
                    .map(|(x, y)| (x as f64, *y as f64))
                    .collect(),
            );
        } else {
            self.audio = Some(TimeSeries::new(
                t,
                44100.,
                samples.iter().map(|x| *x as f64).collect(),
            ))
        }
        if let Some(last_audio) = self.last_audio {
            self.p.plot_series_2d(
                "sampling error",
                "audio timing residual",
                (t - self.start).as_secs_f64(),
                (t - last_audio).as_secs_f64() - samples.len() as f64 / 44100.,
            );
        }
        self.last_audio = Some(t);
        AudioOutput::VoiceEnable(VoiceEnable { audio_enable: () })
    }
    fn video(&mut self, i: VideoInput) -> VideoOutput {
        self.video.push_back(match i {
            VideoInput::Faces(Faces { faces: (t, faces) }) => (t, faces),
        });
        if let Some(audio) = &mut self.audio {
            loop {
                if let Some((t, _)) = self.video.front() {
                    match audio.pop(*t, Duration::from_secs_f64(1.0/self.fps as f64)) {
                        PopRet::TooEarly => {
                            self.video.pop_front().unwrap();
                        },
                        PopRet::Some(audio) => {
                            let (t, faces) = self.video.pop_front().unwrap();
                            let l = audio.len();
                            self.p.plot_series_2d("audio samples per frame", "",
                                (t - self.start).as_secs_f64(), l as f64
                            );
                            for (i, face) in faces.into_iter().enumerate() {
                                self.p.plot_image(
                                    &format!("face[{}]", i),
                                    face.image,
                                    ImageCompression::Lossless,
                                );
                            }
                        },
                        PopRet::TooLate => {
                            break;
                        },
                    }
                } else {
                    break;
                }
            }
        }
        VideoOutput::FacesEnable(FacesEnable { faces_enable: () })
    }
}
