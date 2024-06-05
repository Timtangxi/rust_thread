use std::io;
use rand::Rng;
use std::cmp::Ordering;

fn main() {

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