use mnet_lib::{Place, GraphMaker, GraphRunner, Printer};
use mnet_macro::MnetPlace;

//use rand;
//struct Type1;
//struct Type2;
//enum Letter {
//    A(Type1),
//    B(Type2),
//}
//#[derive(MnetPlace)]
//#[mnet_place(my_enum_function, Letter, Letter)]
//#[mnet_place_enum(Letter::A, Type1, Letter::B, Type2)]
//struct MyEnumPlace;
//impl MyEnumPlace {
//    fn my_enum_function(&self, _: &Printer, _x: Letter) -> Letter {
//        if rand::prelude::random() {
//            Letter::A(Type1{})
//        } else {
//            Letter::B(Type2{})
//        }
//    }
//}

use nokhwa::{Camera, CameraFormat, FrameFormat};
use image::RgbImage;

#[derive(MnetPlace)]
#[mnet_place(f, (), RgbImage)]
struct CameraReader {
    camera: Camera,
}
impl CameraReader {
    fn make() ->Self {
        let width = 1920_u32;
        let height = 1080_u32;
        let mut cam = Camera::new(0, Some(CameraFormat::new_from(width, height, FrameFormat::MJPEG, 30))).unwrap();
        cam.open_stream().unwrap();
        Self {
            camera: cam,
        }
    }
    fn f(&mut self, _p: &Printer, _: ()) -> RgbImage {
        let frame = self.camera.frame().unwrap();
        _p.println("got Image!");
        RgbImage::from_vec(frame.width(), frame.height(), frame.into_vec()).unwrap()
    }
}

use show_image::{ImageView, ImageInfo, create_window, WindowProxy};

#[derive(MnetPlace)]
#[mnet_place(f, RgbImage, ())]
struct ImagePlotter {
    window: WindowProxy,
}
impl ImagePlotter {
    fn make(name: &str) -> Self {
        Self {
            window: create_window(name, Default::default()).unwrap(),
        }
    }
    fn f(&mut self, _p: &Printer, image: RgbImage) {
        self.window.set_image("image",
            ImageView::new(ImageInfo::rgb8(image.width(), image.height()), image.as_raw())
        ).unwrap();
        _p.println("plot Image!");
    }
}

#[show_image::main]
fn main() {
    let mut g = GraphMaker::make(); g
        .set_start_tokens::<()>("Start", vec![()])
        .edge_to_place("Start", "CameraRead")
        .add_place("CameraRead", Box::new(CameraReader::make()))
        .place_to_edge("CameraRead", "Image")
        .add_edge::<RgbImage>("Image")
        .edge_to_place("Image", "PlotImage")
        .add_place("PlotImage", Box::new(ImagePlotter::make("Camera")))
        .place_to_edge("PlotImage", "Start")
    ;
    let e = GraphRunner::from_maker(g).run();
    println!("{:#?}", e);
}
