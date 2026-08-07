extern crate gccjit;
use std::io;
use std::default::Default;
use std::mem;

use gccjit::ToRValue;

const MEMORY_SIZE: u64 = 1000;

#[derive(Copy, Clone)]
pub enum Op {
    Inc,
    Dec,
    ShiftLeft,
    ShiftRight,
    BranchLeft,
    BranchRight,
    Input,
    Output
}

fn main() {
    let context = gccjit::Context::default();
    context.set_optimization_level(gccjit::OptimizationLevel::Limited);
    let stdin = std::io::stdin();
    let ops = match read_ops(stdin) {
        Ok(v) => v,
        Err(e) => panic!("error: {}", e)
    };
    if !codegen(&ops[..], &context) {
        panic!("unbalanced brackets");
    }

    let result = context.compile();
    let main_result = result.get_function("bf_main");
    let main : extern "C" fn() =
        if !main_result.is_null() {
            unsafe { mem::transmute(main_result) }
        }
        else {
           panic!("failed to codegen")
        };
    main();
}

fn read_ops<R: io::Read>(mut reader: R) -> Result<Vec<Op>, io::Error> {
    let mut buf = String::new();
    let mut ops = vec![];
    let _ = try!(reader.read_to_string(&mut buf));
    for c in buf.chars() {
        match c {
            '+' => ops.push(Op::Inc),
            '-' => ops.push(Op::Dec),
            '<' => ops.push(Op::ShiftLeft),
            '>' => ops.push(Op::ShiftRight),
            '[' => ops.push(Op::BranchLeft),
            ']' => ops.push(Op::BranchRight),
            ',' => ops.push(Op::Input),
            '.' => ops.push(Op::Output),
            _ => {}
        }
    }
    Ok(ops)
}

fn codegen<'a, 'ctx>(ops: &[Op], context: &'a gccjit::Context<'ctx>) -> bool {
    // first we set up the function so that it has signature () -> void.
    let void_ty = context.new_type::<()>().unwrap();
    let char_ty = context.new_type::<u8>().unwrap();
    let int_ty = context.new_type::<i32>().unwrap();
    // before we get started - get a reference to getchar, putchar, and memset.
    let getchar = context.new_function(None,
                                       gccjit::FunctionType::Extern,
                                       char_ty,
                                       &[],
                                       "getchar",
                                       false).unwrap();
    let parameter = context.new_parameter(None, char_ty, "c").unwrap();
    let putchar = context.new_function(None,
                                       gccjit::FunctionType::Extern,
                                       void_ty,
                                       &[parameter],
                                       "putchar",
                                       false).unwrap();
    let memory_ty = context.new_array_type(None, char_ty, MEMORY_SIZE).unwrap();
    // memset definition - going to cheat a little bit and not give the C definition since
    // gcc's backend doesn't have C's notion of implicit type conversions (i.e. unsigned char[] to void*)
    let char_ptr = context.new_type::<u8>().unwrap().make_pointer().unwrap();
    let void_param = context.new_parameter(None, char_ptr, "ptr").unwrap();
    // also here - we're lying a bit and saying that int == size_t. This obviously isn't always true
    // but it's good enough for this toy program.
    let size_t_param = context.new_parameter(None, int_ty, "size").unwrap();
    let int_param = context.new_parameter(None, int_ty, "num").unwrap();
    let void_ptr_ty = context.new_type::<*mut ()>().unwrap();
    let memset = context.new_function(None,
                                      gccjit::FunctionType::Extern,
                                      void_ptr_ty,
                                      &[void_param, int_param, size_t_param],
                                      "memset",
                                      false).unwrap();

    let brainf_main = context.new_function(None, gccjit::FunctionType::Exported, void_ty, &[], "bf_main", false).unwrap();
    // next, we set up the brainfuck memory array.
    let size = context.new_rvalue_from_int(int_ty, MEMORY_SIZE as _).unwrap();
    let array = brainf_main.new_local(None, memory_ty, "memory").unwrap();
    let memory_ptr = brainf_main.new_local(None, int_ty, "memory_ptr").unwrap();
    let mut current_block = brainf_main.new_block("entry_block").unwrap();
    // now we have to zero out the giant buffer we just allocated on the stack.
    let zero_access = context.new_array_access(None, array.to_rvalue(), context.new_rvalue_zero(int_ty).unwrap()).unwrap();
    // A function call that is done for its side effects must be sent to add_eval.
    current_block.add_eval(None, context.new_call(None, memset, &[zero_access.get_address(None).unwrap(), context.new_rvalue_zero(int_ty).unwrap(), size]).unwrap());
    let mut block_stack = vec![];
    let mut blocks = 0;
    for op in ops.iter() {
        match *op {
            Op::Inc => {
                // memory[ptr] += 1
                let access = context.new_array_access(None, array.to_rvalue(), memory_ptr.to_rvalue()).unwrap();
                current_block.add_assignment_op(None, access, gccjit::BinaryOp::Plus, context.new_rvalue_one(char_ty).unwrap());
            },
            Op::Dec => {
                // memory[ptr] -=
                let access = context.new_array_access(None, array.to_rvalue(), memory_ptr.to_rvalue()).unwrap();
                current_block.add_assignment_op(None, access, gccjit::BinaryOp::Minus, context.new_rvalue_one(char_ty).unwrap());
            },
            Op::ShiftLeft => {
                // ptr -= 1                
                current_block.add_assignment_op(None, memory_ptr, gccjit::BinaryOp::Minus, context.new_rvalue_one(int_ty).unwrap());
            },
            Op::ShiftRight => {
                // ptr += 1
                current_block.add_assignment_op(None, memory_ptr, gccjit::BinaryOp::Plus, context.new_rvalue_one(int_ty).unwrap());
            },
            Op::BranchLeft => {
                // this is the opening bracket. This represents the start of two
                // new blocks. The block that is directly ahead of us (and the
                // one that will be codegen'd next) is branched to when memory[ptr]
                // is not zero. We will create the other block now but will put
                // it on the block stack.
                let cond_block = brainf_main.new_block(&*format!("block{}", blocks)).unwrap();
                let true_block = brainf_main.new_block(&*format!("block{}", blocks + 1)).unwrap();
                let false_block = brainf_main.new_block(&*format!("block{}", blocks + 2)).unwrap();
                blocks += 3;
                // end the current block with a jump to the conditional block.
                current_block.end_with_jump(None, cond_block);

                current_block = cond_block;

                // end the condition block with a jump to the true_block if
                // mem[ptr] != 0, false_block otherwise
                let access = context.new_array_access(None, array.to_rvalue(), memory_ptr.to_rvalue()).unwrap().to_rvalue();
                let cond = context.new_comparison(None,
                                                  gccjit::ComparisonOp::NotEquals,
                                                  access,
                                                  context.new_rvalue_zero(char_ty).unwrap()).unwrap();
                current_block.end_with_conditional(None, cond, true_block, false_block);
                // now we are going to codegen the true branch.
                current_block = true_block;
                // we push the cond block and false block onto the stack
                // so branchright knows where to jump.
                block_stack.push((cond_block, false_block));
            }
            Op::BranchRight => {
                // end the current block with a jump to the cond block on the
                // stack.
                let (cond, next_block) = match block_stack.pop() {
                    Some(t) => t,
                    None => return false
                };
                current_block.end_with_jump(None, cond);
                // the next block is next_block.
                current_block = next_block;
            }
            Op::Input => {
                let access = context.new_array_access(None, array.to_rvalue(), memory_ptr.to_rvalue()).unwrap();
                let chr = context.new_call(None, getchar, &[]).unwrap();
                current_block.add_assignment(None, access, chr);
            },
            Op::Output => {
                let access = context.new_array_access(None, array.to_rvalue(), memory_ptr.to_rvalue()).unwrap();
                let call = context.new_call(None, putchar, &[access.to_rvalue()]).unwrap();
                current_block.add_eval(None, call);
            }
        }
    }
    // this program is only valid if the block stack is zero.
    if block_stack.len() != 0 {
        return false;
    }
    // finish off the last block with a ret.
    current_block.end_with_void_return(None);
    true
}
