use std::io;
use rand::Rng;
use std::cmp::Ordering;

fn geussed_play() {

    println!("Guess the number!");
    let secret_number = rand::thread_rng().gen_range(1..=100);
    loop {
        println!("Please input your guess.");

        let mut guess = String::new();
        let f = io::stdin().read_line(&mut guess);
        if let Err(_f) = f {
            println!("Failed to read line");
        }

        // let guess: u32 = guess.trim().parse().expect("Failed to read line");
        let guess: u32 = match guess.trim().parse() {
            Ok(num) => num,
            Err(_) => continue,
        };

        match guess.cmp(&secret_number) {
            Ordering::Less => println!("You guessed: {guess}, But too small!"),
            Ordering::Greater => println!("You guessed: {guess}, But too big!"),
            Ordering::Equal => {
                println!("You guessed: {guess}, You win!");
                break;
            }
        }
    }
}

const THREE_HOURS_IN_SECONDS: u32 = 60 * 60 * 3;
fn num_run() {

    // 数组
    let a: [i32; 5] = [1, 2, 3, 4, 5];
    println!("Please enter an array index.");
    let mut index = String::new();
    io::stdin()
        .read_line(&mut index)
        .expect("Failed to read line");
    let index: usize = index
        .trim()
        .parse()
        .expect("Index entered was not a number");
    println!("The value of the element at index {index} is: {}", a[index]);

    // 元组
    let tup: (i32, f64, u8) = (500, 6.4, 1);
    println!("The value of x is: {}", tup.0);

    // 隐藏
    let mut x: i32 = 5;
    println!("The value of x is: {x}");
    x = 6;
    println!("The value of x is: {THREE_HOURS_IN_SECONDS}");
    let x: i32 = x + 1;
    {
        let mut x: i32 = x * 2;
        x =  5;
        println!("The value of x in the inner scope is: {x}");
    }
    println!("The value of x in the inner scope is: {x}");
    
    // 字符串
    let spaces: &str = "   ";
    let spaces = spaces.len();
    println!("The value of x in the inner scope is: {spaces}");
}

fn main(){
    
}