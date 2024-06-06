pub mod std_io;
struct User {
    active: bool,
    username: String,
    email: String,
    sign_in_count: u64,
}

fn build_user(email: &mut String, username: &mut String) -> User {
    let user_test = User{
        active: true,
        username: username.to_string(),
        email: email.to_string(),
        sign_in_count: 0,
    };
    return user_test;
}
struct Color(i32, i32, i32);

fn struct_test(){
    { 
        let mut user1 = User {
            active: true,
            username: String::from("someusername123"),
            email: String::from("someone@example.com"),
            sign_in_count: 1,
        };
        let user2 = build_user(&mut user1.email, &mut user1.username);
        println!("{} {} {} {}", user1.active, user1.email, user2.username, user2.sign_in_count);
    }
    {
        let black = Color(1, 2, 3);
        println!("{} {} {}", black.0, black.1, black.2);
    }
}

#[derive(Debug)]
struct Rectangle {
    width: u32,
    height: u32,
}

impl Rectangle {
    fn area(&self) -> u32 {
        self.width * self.height
    }
    fn width(&self) -> bool {
        self.width > 0
    }
    fn can_hold(&self, other: &Rectangle) -> bool {
        self.width > other.width && self.height > other.height
    }
    fn square(size: u32, size1: u32) -> Self {
        Self {
            width: size,
            height: size,
        }
    }

}
fn main() {
    // 结构体方法
    {
        struct_test();
        let rect1 = Rectangle {
            width: 30,
            height: 50,
        };
        println!(
            "The area of the rectangle is {} square pixels.",
            rect1.area()  // object->something() 就像 (*object).something() 一样。
        );
        if rect1.width() {
            println!("The rectangle has a nonzero width; it is {}", rect1.width);
        }
        println!("Can rect1 hold rect2? {}", rect1.can_hold(&rect1));
        let rect2 = Rectangle::square(30, 50);
        println!("Can rect1 hold rect2? {}", rect2.can_hold(&rect2));
    }

    // 枚举
    {
        std_io::geussed_play();
    }
}