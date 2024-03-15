// fn fun(b: i32 , a: i32)-> i32
// {
//     return a+b;
// }
// fn name() -> i32 {
//      1
// }
fn main() {
        enum Book {
            Papery(u32),
            Electronic(String)
        }
        let book: Book = Book::Electronic(String::from("url"));
        if let Book::Papery(index) = book {
            println!("Papery {}", index);
        } else {
            println!("Not papery book");
        }
    }
// fn main() {
//     let a: &str = "uoad";
//     let mut test : i32 = 5; 
//     test = 6;
//     let x: i32 =  5;
//     let x: i32 = x + 1;
//     println!("{} {} {} {} {}", a, test, x, fun(5, 6), name());
// }

/* 
fn main() { 
    let a = [10, 20, 30, 40, 50]; 
    for i in a.iter() { 
    println!("值为 : {}", i); 
    } 
}
*/