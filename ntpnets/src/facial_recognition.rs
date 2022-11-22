use tensorflow::{Tensor, Session, SessionOptions, Graph, ImportGraphDefOptions, SessionRunArgs};
use image::{RgbImage, Rgb};
use std::env;
use std::path::Path;
use std::fs::File;
use std::io::Read;
use std::iter::zip;
use std::time::Instant;

use ntpnet_macro;
use ntpnet_lib;
use plotmux::plotsink::{PlotSink, ImageCompression};

#[derive(ntpnet_macro::TransitionInputTokens)]
struct Image {
    image: (Instant, RgbImage),
    _e: (),
}

#[derive(Debug)]
pub struct BBox {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub _prob: f32,
}

pub struct Face {
    bbox: BBox,
    pub image: RgbImage,
}

#[derive(ntpnet_macro::TransitionOutputTokens)]
struct Faces {
    faces: (Instant, Vec<Face>),
    next_image: (),
}

#[derive(ntpnet_macro::Transition)]
#[ntpnet_transition(f: Input(Image) -> Output(Faces))]
pub struct FacialRecognition {
    p: PlotSink,
    graph: Graph,
    session: Session,
    min_size: Tensor<f32>,
    thresholds: Tensor<f32>,
    factor: Tensor<f32>,
}
impl FacialRecognition {
    pub fn maker(mut plotsink: PlotSink) -> ntpnet_lib::TransitionMaker {
        Box::new(move || {
            let model = {
                let path = env::current_exe().unwrap().as_path().parent().unwrap().parent().unwrap().join(Path::new("data/mtcnn.pb"));
                plotsink.println2("model", &format!("{:?}", path));
                let mut f = File::open(path).expect("failed to open file");
                let mut buffer = vec![];
                f.read_to_end(&mut buffer).expect("buffer overflow");
                buffer
            };
            let mut g = Graph::new();
            g.import_graph_def(&model, &ImportGraphDefOptions::new()).unwrap();
            Box::new(FacialRecognition {
                p: plotsink,
                session: Session::new(&SessionOptions::new(), &g).unwrap(),
                graph: g,
                min_size: Tensor::new(&[]).with_values(&[40f32]).unwrap(),
                thresholds: Tensor::new(&[3]).with_values(&[0.6f32, 0.7f32, 0.7f32]).unwrap(),
                factor: Tensor::new(&[]).with_values(&[0.709f32]).unwrap(),
            })
        })
    }
    fn f(&mut self, i: Input) -> Output {
        let (t, image) = match i { Input::Image(Image { _e: _, image: (t, image) }) => (t, image) };
        let image_f_bgr = image.pixels().flat_map(|rgb| [rgb[2] as f32, rgb[1] as f32, rgb[0] as f32]).collect::<Vec<f32>>();
        let input = Tensor::new(&[image.height() as u64, image.width() as u64, 3]).with_values(&image_f_bgr).unwrap();
        let mut run_args = SessionRunArgs::new();
        run_args.add_feed(&self.graph.operation_by_name_required("min_size").unwrap(), 0, &self.min_size);
        run_args.add_feed(&self.graph.operation_by_name_required("thresholds").unwrap(), 0, &self.thresholds);
        run_args.add_feed(&self.graph.operation_by_name_required("factor").unwrap(), 0, &self.factor);
        run_args.add_feed(&self.graph.operation_by_name_required("input").unwrap(), 0, &input);
        let bbox = run_args.request_fetch(&self.graph.operation_by_name_required("box").unwrap(), 0);
        let prob = run_args.request_fetch(&self.graph.operation_by_name_required("prob").unwrap(), 0);
        self.session.run(&mut run_args).unwrap();
        let bboxes = zip(run_args.fetch(bbox).unwrap().chunks_exact(4), run_args.fetch(prob).iter()).map(|(bbox, prob)| {
            assert!(prob.len() >= 1);
            BBox {
                y1: bbox[0],
                x1: bbox[1],
                y2: bbox[2],
                x2: bbox[3],
                _prob: prob[0],
            }
        }).collect::<Vec<_>>();
        let mut faces = vec![];
        let mut faces_image = RgbImage::from_pixel(image.width(), image.height(), Rgb::<u8>::from([0, 0, 0]));
        for bbox in bboxes.into_iter() {
            let mut face = RgbImage::from_pixel(
                bbox.x2 as u32 - bbox.x1 as u32,
                bbox.y2 as u32 - bbox.y1 as u32,
                Rgb::<u8>::from([0, 0, 0])
            );
            for (x, i) in (bbox.x1 as u32 .. bbox.x2 as u32).enumerate() {
                for (y, j) in (bbox.y1 as u32 .. bbox.y2 as u32).enumerate() {
                    let i = i.clamp(0, image.width()-1);
                    let j = j.clamp(0, image.height()-1);
                    faces_image.put_pixel(i, j, *image.get_pixel(i, j));
                    face.put_pixel(x as _, y as _, *image.get_pixel(i, j));
                }
            }
            faces.push(Face {
                bbox: bbox,
                image: face,
            });
        }
        self.p.plot_image("faces", faces_image, ImageCompression::Lossless);
        Output::Faces(Faces{faces: (t, faces), next_image: ()})
    }
}
