use mnet_lib::{Place, GraphMaker};
use mnet_macro::MnetPlace;
use rand;

struct Type1;
struct Type2;
enum Letter {
    A(Type1),
    B(Type2),
}
#[derive(MnetPlace)]
#[mnet_place(my_enum_function, Letter, Letter)]
#[mnet_place_enum(Letter::A, Type1, Letter::B, Type2)]
struct MyEnumPlace;
impl MyEnumPlace {
    fn my_enum_function(&self, _x: Letter) -> Letter {
        if rand::prelude::random() {
            Letter::A(Type1{})
        } else {
            Letter::B(Type2{})
        }
    }
}

#[derive(MnetPlace)]
#[mnet_place(my_increment_function, i32, i32)]
struct MyIncrementPlace;
impl MyIncrementPlace {
    fn my_increment_function(&self, x: i32) -> i32 {
        x + 1
    }
}

#[derive(MnetPlace)]
#[mnet_place(f, (), ())]
struct NonePlace;
impl NonePlace {
    fn f(&self, _: ()) -> () {
        println!("none place\n");
    }
}

fn main() {
   let _g = GraphMaker::make()
        .add_place("P1".into(), Box::new(NonePlace{}))
        .add_place("P2".into(), Box::new(NonePlace{}))
        .set_start_tokens::<()>("E0".into(), vec![()])
        .add_edge::<()>("E1".into())
        .add_edge::<()>("Drop".into())
        .place_to_edge("P1".into(), "E1".into())
        .edge_to_place("E0".into(), "P1".into())
        .edge_to_place("E1".into(), "P2".into())
        .place_to_edge("P2".into(), "Drop".into())
        .to_runner().run();
}
