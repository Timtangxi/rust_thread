use std::io;
use rand::Rng;
use std::cmp::Ordering;

/**************************
 * rust基础知识
 */

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
        // if guess < secret_number {
        //     println!("You guessed: {guess}, But too small!");
        // }
        // else if guess > secret_number {
        //    println!("You guessed: {guess}, But too big!");
        // }
        // else {
        //     println!("You guessed: {guess}, You win!");
        //     break;
        // }
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

fn five(x:i32) -> i32 {
    return x + 5;
}
fn string_test(){
    // 代码块
    {
        let y = {
            let x = 3;
            x + 1
        };
        println!("The value of y is: {y}");
    }

    // 函数
    {
        let five_num = five(5);
        println!("The value of y is: {}", five_num);
    }

    // 判断语句
    {
        let number: i32 = 3;
        if number < 5 {
            println!("condition was true");
        } else {
            println!("condition was false");
        }
    }

    // 判断语句返回值
    {
        let condition = true;
        let number = if condition { 5 } else { 6 };
        println!("The value of number is: {number}");
    }

    // while 循环
    {
        let mut number: i8 = 3;
        while number != 0 {
            println!("{number}!");
            number -= 1;
        }
        println!("LIFTOFF!!!");
    }
    
    // for 循环
    {
        let a = [10, 20, 30, 40, 50];
        for i in 0..5 {
            println!("the value is: {}", a[i]);
        }
    }

    // 字符串拼接
    {
        let mut s = String::from("hello");
        s.push_str(", world!"); // push_str() 在字符串后追加字面值
        println!("{}", s); // 将打印 `hello, world!`
    }

    // 字符串所有权问题
    {
        let s1 = String::from("hello");
        let s2 = s1;
        // println!("{}, world!", s1);  s1 已经被取消掉，无用，栈顶指针给了s2
        println!("{}, world!", s2);
    }

    // 字符串clone 
    {
        let s1 = String::from("hello");
        let s2 = s1.clone();
        println!("s1 = {}, s2 = {}", s1, s2);   // 将s1克隆给了s2,s1都可用

        // 作用于在函数的使用
        let s = String::from("hello");          // s 进入作用域
        // takes_ownership(s);                  // s 的值移动到函数里 ...
        // println!("s = {}",s);                // ... 所以到这里不再有效
        takes_ownership(s.clone());// s 的clone移动到函数里 ...
        println!("s = {}",s);                   // ... 所以到这里有效

        let x = 5;                          // x 进入作用域
        makes_copy(x);                  // x 应该移动函数里，但 i32 是 Copy 的，所以在后面可继续使用 x。这里，x 先移出了作用域，然后是 s。但因为 s 的值已被移走，
    }                                

    {
        let s1 = gives_ownership();         // gives_ownership 将返回值// 转移给 s1
        let s2 = String::from("hello");     // s2 进入作用域
        let s3 = takes_and_gives_back(s2);  // s2 被移动到 takes_and_gives_back 中，它也将返回值移给 s3
        println!("s1 = {}, s3 = {}",s1, s3);                   // 将s2移走，s2此处无效
    }// 这里，s3 移出作用域并被丢弃。s2 也移出作用域，但已被移走，所以什么也不会发生。s1 离开作用域并被丢弃

    {
        let s1 = String::from("hello");
        let (s2, len) = calculate_length(s1);
        println!("The length of '{}' is {}.", s2, len);
    }
  
                    
}

fn calculate_length(s: String) -> (String, usize) {
    let length = s.len(); // len() 返回字符串的长度
    (s, length)
}

fn gives_ownership() -> String {             // gives_ownership 会将返回值移动给调用它的函数
    let some_string = String::from("yours"); // some_string 进入作用域。
    some_string                              // 返回 some_string 并移出给调用的函数
}

// takes_and_gives_back 将传入字符串并返回该值
fn takes_and_gives_back(a_string: String) -> String { // a_string 进入作用域
    a_string  // 返回 a_string 并移出给调用的函数
}

fn takes_ownership(some_string: String) { // some_string 进入作用域
    println!("{}", some_string);
} // 这里，some_string 移出作用域并调用 `drop` 方法。占用的内存被释放

fn makes_copy(some_integer: i32) { // some_integer 进入作用域
    println!("{}", some_integer);
} // 这里，some_integer 移出作用域。没有特殊之处

fn calculate_length_test(s: &String) -> usize {
    s.len()
}

fn change(some_string: &mut String) {
    some_string.push_str(", world");
}
fn main() {

    {
        let s1 = String::from("hello"); 
        let len = calculate_length_test(&s1);      //  引用（reference）像一个指针，因为它是一个地址
        println!("The length of '{}' is {}.", s1, len);   //  传入地址，s1可用

        let mut s = String::from("hello");
        change(&mut s);
    }

    {
        let mut s = String::from("hello");
        {
            let r1 = &mut s;
            let r2 = &mut s;    //
            // println!("The length of '{}' is {}.", r1, r2);   //  同一作用域不能重复引用，可改变
        } // r1 在这里离开了作用域，所以我们完全可以创建一个新的引用
        let r2 = &mut s;   
        println!("The length of is {}.", r2);   //  同一作用域不能重复引用 
    }

    {
        let mut s = String::from("hello");
        let r1 = &s;
        let r2 = &s;    //
        // println!("The length of '{}' is {}.", r1, r2);   //  同一作用域不能重复引用
        let r3 = &s;   
        println!("The length of is {}, {}, {}",r1, r2, r3);  // 同一作用域能重复引用，不可改变
    }  // 类似于不能同时对同一地址的数据做出改动，但是仅是引用不可改动的，可重复引用。


    {
        let mut s = String::from("hello1 world");
        {
            let word = first_word(&s); // word 的值为 5
            println!("The length of is {}, {}",word, s);  // 同一作用域能重复引用，不可改变
        }
        change(&mut s);
        println!("The length of is {}", s);  // 同一作用域能重复引用，不可改变
        s.clear(); 
    }
}


fn first_word(s: &String) -> &str {
    let bytes = s.as_bytes();// 用 as_bytes 方法将 String 转化为字节数组。

    for (i, &item) in bytes.iter().enumerate() {
        if item == b' ' {
            return &s[0..i];
        }
    }
    &s[..]
}
    