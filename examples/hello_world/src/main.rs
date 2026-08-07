extern crate gccjit;

use gccjit::Context;
use gccjit::FunctionType;

use std::default::Default;
use std::mem;


fn main() {
    let context = Context::default();
    let void_ty = context.new_type::<()>().unwrap();
    let fun = context.new_function(None,
                                   FunctionType::Exported,
                                   void_ty,
                                   &[],
                                   "hello",
                                   false).unwrap();
    let block = fun.new_block("main_block").unwrap();
    let function_ptr = context.new_function_pointer_type(None,
                                                         void_ty,
                                                         &[],
                                                         false).unwrap();
    let ptr = context.new_rvalue_from_ptr(function_ptr, say_hello as *mut ()).unwrap();
    let call = context.new_call_through_ptr(None, ptr, &[]).unwrap();
    block.add_eval(None, call);
    block.end_with_void_return(None);
    let result = context.compile().unwrap();
    let hello = result.get_function("hello").unwrap();
    let hello_fn : extern "C" fn() = unsafe { mem::transmute(hello) };
    hello_fn();
}

extern "C" fn say_hello() {
    println!("hello, world!");
}
