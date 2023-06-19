use image::{Rgb, RgbImage};
use itertools::izip;
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Instant;
use tensorflow::{Graph, ImportGraphDefOptions, Session, SessionOptions, SessionRunArgs, Tensor};

use ntpnet;
use plotmux::plotsink::{ImageCompression, PlotSink};

#[derive(ntpnet::TransitionInputTokensMacro)]
struct Image {
    image: (Instant, RgbImage),
    _enable: (),
}

#[derive(Debug)]
pub struct Landmarks {
    pub left_eye: (f32, f32),
    pub right_eye: (f32, f32),
    pub nose: (f32, f32),
    pub left_mouth: (f32, f32),
    pub right_mouth: (f32, f32),
}
impl Landmarks {
    fn make(landmarks: &[f32]) -> Landmarks {
        Landmarks {
            left_eye: (landmarks[5], landmarks[0]),
            right_eye: (landmarks[6], landmarks[1]),
            nose: (landmarks[7], landmarks[2]),
            left_mouth: (landmarks[8], landmarks[3]),
            right_mouth: (landmarks[9], landmarks[4]),
        }
    }
}
fn draw_point((x, y): &(f32, f32), image: &mut image::RgbImage) {
    let x = f32::max(5., *x) as u32;
    let y = f32::max(5., *y) as u32;
    for i in x - 5..x + 5 {
        for j in y - 5..y + 5 {
            let i = (i as u32).clamp(0, image.width() - 1);
            let j = (j as u32).clamp(0, image.height() - 1);
            image.put_pixel(i, j, image::Rgb([0, 255, 0]));
        }
    }
}

#[derive(Debug)]
pub struct FaceBBox {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub prob: f32,
    pub landmarks: Landmarks,
}

#[derive(ntpnet::TransitionOutputTokensMacro)]
struct Faces {
    faces: (Instant, RgbImage, Vec<FaceBBox>),
    done: (),
}

#[derive(ntpnet::Transition)]
#[ntpnet_transition(f: Input(Image) -> Output(Faces))]
pub struct FacialDetection {
    p: PlotSink,
    graph: Graph,
    session: Session,
    min_size: Tensor<f32>,
    thresholds: Tensor<f32>,
    factor: Tensor<f32>,
    _t0: Instant,
}
impl FacialDetection {
    pub fn maker(mut plotsink: PlotSink) -> ntpnet::TransitionMaker {
        Box::new(|| {
            let model = {
                let path = env::current_exe()
                    .unwrap()
                    .as_path()
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .join(Path::new("data/mtcnn.pb"));
                plotsink.println2("model", &format!("{:?}", path));
                let mut f = File::open(path).expect("failed to open file");
                let mut buffer = vec![];
                f.read_to_end(&mut buffer).expect("buffer overflow");
                buffer
            };
            let mut g = Graph::new();
            g.import_graph_def(&model, &ImportGraphDefOptions::new())
                .unwrap();

            Box::new(Self {
                p: plotsink,
                session: Session::new(&SessionOptions::new(), &g).unwrap(),
                graph: g,
                min_size: Tensor::new(&[]).with_values(&[40f32]).unwrap(),
                thresholds: Tensor::new(&[3])
                    .with_values(&[0.6f32, 0.7f32, 0.7f32])
                    .unwrap(),
                factor: Tensor::new(&[]).with_values(&[0.709f32]).unwrap(),
                _t0: std::time::Instant::now(),
            })
        })
    }
    fn f(&mut self, i: Input) -> Output {
        let (t, image) = match i {
            Input::Image(Image { image, .. }) => image,
        };
        let image_f_bgr = image
            .pixels()
            .flat_map(|rgb| [rgb[2] as f32, rgb[1] as f32, rgb[0] as f32])
            .collect::<Vec<f32>>();
        let input = Tensor::new(&[image.height() as u64, image.width() as u64, 3])
            .with_values(&image_f_bgr)
            .unwrap();
        let mut run_args = SessionRunArgs::new();
        run_args.add_feed(
            &self.graph.operation_by_name_required("min_size").unwrap(),
            0,
            &self.min_size,
        );
        run_args.add_feed(
            &self.graph.operation_by_name_required("thresholds").unwrap(),
            0,
            &self.thresholds,
        );
        run_args.add_feed(
            &self.graph.operation_by_name_required("factor").unwrap(),
            0,
            &self.factor,
        );
        run_args.add_feed(
            &self.graph.operation_by_name_required("input").unwrap(),
            0,
            &input,
        );
        let bbox =
            run_args.request_fetch(&self.graph.operation_by_name_required("box").unwrap(), 0);
        let prob =
            run_args.request_fetch(&self.graph.operation_by_name_required("prob").unwrap(), 0);
        let landmarks = run_args.request_fetch(
            &self.graph.operation_by_name_required("landmarks").unwrap(),
            0,
        );
        self.session.run(&mut run_args).unwrap();
        let bboxes = izip!(
            run_args.fetch(bbox).unwrap().chunks_exact(4),
            run_args.fetch(prob).iter(),
            run_args.fetch::<f32>(landmarks).unwrap().chunks_exact(10),
        )
        .map(|(bbox, prob, landmarks)| {
            assert!(prob.len() >= 1);
            FaceBBox {
                y1: bbox[0],
                x1: bbox[1],
                y2: bbox[2],
                x2: bbox[3],
                prob: prob[0],
                landmarks: Landmarks::make(landmarks),
            }
        })
        .collect::<Vec<_>>();
        let mut faces =
            RgbImage::from_pixel(image.width(), image.height(), Rgb::<u8>::from([0, 0, 0]));
        for bbox in &bboxes {
            for i in bbox.x1 as u32..bbox.x2 as u32 {
                for j in bbox.y1 as u32..bbox.y2 as u32 {
                    let i = i.clamp(0, image.width() - 1);
                    let j = j.clamp(0, image.height() - 1);
                    faces.put_pixel(i, j, *image.get_pixel(i, j));
                }
            }
            draw_point(&bbox.landmarks.left_eye, &mut faces);
            draw_point(&bbox.landmarks.right_eye, &mut faces);
            draw_point(&bbox.landmarks.nose, &mut faces);
            draw_point(&bbox.landmarks.left_mouth, &mut faces);
            draw_point(&bbox.landmarks.right_mouth, &mut faces);
            //let mouth_center = (
            //    (bbox.landmarks.right_mouth.0 + bbox.landmarks.left_mouth.0) / 2.,
            //    (bbox.landmarks.nose.1 + bbox.y2) / 2.,
            //);
            //draw_point(&mouth_center, &mut faces);
            //let mut mouth = RgbImage::new(
            //    ((bbox.landmarks.right_mouth.0 - bbox.landmarks.left_mouth.0) / 2.) as u32,
            //    ((bbox.y2 - bbox.landmarks.nose.1) / 4.) as u32,
            //);
            //for i in 0 .. mouth.width() {
            //    for j in 0 .. mouth.height() {
            //        let i2 = (i + mouth_center.0 as u32 - mouth.width() / 2).clamp(0, image.width()-1);
            //        let j2 = (j + mouth_center.1 as u32 - mouth.height() / 2).clamp(0, image.height()-1);
            //        mouth.put_pixel(i, j, *image.get_pixel(i2, j2));
            //    }
            //}
            //let mouth : GrayImage = DynamicImage::ImageRgb8(mouth).into_luma8();
            //let mut bright = 0.;
            //for p in mouth.pixels() {
            //    bright += p[0] as f64;
            //}
            //bright /= mouth.len() as f64;
            //self.p.plot_series_2d("0", "distance", (t - self.t0).as_secs_f64(), bright);
            //self.p.plot_image("0", DynamicImage::ImageLuma8(mouth).into_rgb8(), ImageCompression::Lossless);
        }
        self.p
            .plot_image("faces", faces.clone(), ImageCompression::Lossless);
        Output::Faces(Faces {
            faces: (t, faces, bboxes),
            done: (),
        })
    }
}
