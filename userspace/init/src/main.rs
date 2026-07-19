#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;
use libos::println;

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("Hello from Vain Userspace (Ring 3)!");
    
    let mut v = Vec::new();
    for i in 0..10 {
        v.push(i * 10);
    }
    
    println!("Allocated a vector: {:?}", v);
    println!("Init exiting cleanly...");
    
    0
}
