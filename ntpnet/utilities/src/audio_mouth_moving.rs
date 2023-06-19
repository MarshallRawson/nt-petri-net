use std::time::Instant;

use image::{DynamicImage, GrayImage, Luma};

use ntpnet;
use plotmux::plotsink::{ImageCompression, PlotSink};

use std::collections::VecDeque;

#[derive(ntpnet::TransitionInputTokensMacro)]
struct AudioFFT {
    audio: (Instant, Vec<i16>, Vec<(f64, f64)>),
}
#[derive(ntpnet::TransitionInputTokensMacro)]
struct MouthMoving {
    mouth_moving: (Instant, Vec<f64>),
}
#[derive(ntpnet::TransitionOutputTokensMacro)]
struct AudioEnable {
    audio_done: (),
}
const AUDIO_RET: AudioOutput = AudioOutput::AudioEnable(AudioEnable { audio_done: () });
#[derive(ntpnet::TransitionOutputTokensMacro)]
struct MouthMovingEnable {
    mouth_moving_done: (),
}

const AUDIO_BUFFER: usize = 100;

#[derive(ntpnet::Transition)]
#[ntpnet_transition(audio: AudioInput(AudioFFT) -> AudioOutput(AudioEnable))]
#[ntpnet_transition(mouth_moving: MouthMovingInput(MouthMoving) -> MouthMovingOutput(MouthMovingEnable))]
pub struct AudioMouthMoving {
    p: PlotSink,
    tb: Option<Instant>,
    waterfall: VecDeque<Vec<f64>>,
    mouth_moving: VecDeque<Vec<f64>>,
}
impl AudioMouthMoving {
    pub fn maker(p: PlotSink) -> ntpnet::TransitionMaker {
        Box::new(|| {
            Box::new(Self {
                p: p,
                tb: None,
                waterfall: {
                    let mut wf = VecDeque::new();
                    for _ in 0..1000 + AUDIO_BUFFER {
                        wf.push_back(vec![0.; 500]);
                    }
                    wf
                },
                mouth_moving: {
                    let mut wf = VecDeque::new();
                    for _ in 0..1000 {
                        wf.push_back(vec![0.; 500]);
                    }
                    wf
                },
            })
        })
    }
    fn audio(&mut self, i: AudioInput) -> AudioOutput {
        let (t, _, fft_samples) = match i {
            AudioInput::AudioFFT(AudioFFT {
                audio: (t, samples, fft_samples),
            }) => (t, samples, fft_samples),
        };
        if self.tb.is_none() {
            return AUDIO_RET;
        }
        let i = ((t - self.tb.unwrap()).as_secs_f64() / 0.03) as i64 + AUDIO_BUFFER as i64;
        if i < 0 {
            return AUDIO_RET;
        }
        let i = i as usize;
        for (j, (_, amp)) in fft_samples.iter().enumerate() {
            if j > 499 {
                break;
            }
            self.waterfall[i][j] += amp;
        }
        AUDIO_RET
    }
    fn mouth_moving(&mut self, i: MouthMovingInput) -> MouthMovingOutput {
        let (t, mouth_moving) = match i {
            MouthMovingInput::MouthMoving(MouthMoving {
                mouth_moving: (t, samples),
            }) => (t, samples),
        };
        self.tb = Some(t);
        self.p.plot_image(
            "audio_spectrograph",
            {
                let mut wf = GrayImage::new(1000, 500);
                for (i, r) in self.waterfall.iter().enumerate() {
                    if i < AUDIO_BUFFER {
                        continue;
                    }
                    for (j, c) in r.iter().enumerate() {
                        wf.put_pixel(
                            (i - AUDIO_BUFFER) as u32,
                            j as u32,
                            Luma::from([(c / 255.) as u8]),
                        )
                    }
                }
                DynamicImage::ImageLuma8(wf).into_rgb8()
            },
            ImageCompression::Lossless,
        );
        self.waterfall.pop_back();
        self.waterfall.push_front(vec![0.; 500]);
        self.p.plot_image(
            "mouth_moving",
            {
                let mut wf = GrayImage::new(1000, 500);
                for (i, r) in self.mouth_moving.iter().enumerate() {
                    for (j, c) in r.iter().enumerate() {
                        wf.put_pixel(
                            i as u32,
                            j as u32,
                            Luma::from([(f64::clamp(*c, 0., 1.) * 255.) as u8]),
                        )
                    }
                }
                DynamicImage::ImageLuma8(wf).into_rgb8()
            },
            ImageCompression::Lossless,
        );
        if mouth_moving.len() > 0 {
            self.mouth_moving.pop_back();
            self.mouth_moving.push_front(vec![mouth_moving[0]; 500]);
        }
        MouthMovingOutput::MouthMovingEnable(MouthMovingEnable {
            mouth_moving_done: (),
        })
    }
}
