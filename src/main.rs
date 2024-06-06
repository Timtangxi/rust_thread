mod some_test;
pub use some_test::stuct_test;

fn init() -> i32 {
    stuct_test::RST_TURE
}

fn open() -> i32 {
    stuct_test::RST_TURE
}

fn close() -> i32 {
    stuct_test::RST_FALSE
}

mod rst_device_ops {
    pub(crate) use crate::init;
    pub(crate) use crate::open;
    pub(crate) use crate::close;
}

fn main(){
    let a = rst_device_ops::init();
    let b = rst_device_ops::open();
    let c = rst_device_ops::close();
    println!("{a} {b} {c}");
}