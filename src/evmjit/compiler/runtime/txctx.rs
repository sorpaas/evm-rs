#![allow(dead_code)]

use singletonum::{Singleton, SingletonInit};
use inkwell::context::Context;
use inkwell::builder::Builder;
use inkwell::module::Module;
use inkwell::types::StructType;
use inkwell::types::PointerType;
use inkwell::values::PointerValue;
use inkwell::values::FunctionValue;
use inkwell::module::Linkage::*;
use inkwell::AddressSpace;
use evmjit::compiler::runtime::env::EnvDataType;
use llvm_sys::LLVMCallConv::*;
use std::ffi::CString;


#[derive(Debug, Singleton)]

pub struct TransactionContextType {
    txctx_type: StructType,
    txctx_ptr_type: PointerType,
}

unsafe impl Sync for TransactionContextType {}
unsafe impl Send for TransactionContextType {}

impl SingletonInit for TransactionContextType {
    type Init = Context;
    fn init(context: &Context) -> Self {
        let i64_t = context.i64_type();
        let i256_t = context.custom_width_int_type(256);
        let i8_t = context.i8_type();
        let array_of_160_bytes_t = i8_t.array_type(20);
        
        let fields = [i256_t.into(),            // Transaction gas price
                      array_of_160_bytes_t.into(),   // Transaction origin account
                      array_of_160_bytes_t.into(),   // Miner of the block (Coinbase)
                      i64_t.into(),                  // Block number
                      i64_t.into(),                  // block timestamp
                      i64_t.into(),                  // Block gas limit
                      i256_t.into()];                // Block difficulity
        
        let tx_struct = context.opaque_struct_type("evm.txctx");
        tx_struct.set_body(&fields, false);

        TransactionContextType {
            txctx_type : tx_struct,
            txctx_ptr_type : tx_struct.ptr_type(AddressSpace::Generic)
        }
    }
}

impl TransactionContextType {
    pub fn get_type(&self) -> StructType {
        self.txctx_type
    }

    pub fn get_ptr_type(&self) -> PointerType {
        self.txctx_ptr_type
    }
    
    pub fn get_num_fields(&self) -> u32 {
        self.get_type().count_fields()
    }

    pub fn is_transaction_context_type(a_struct: &StructType) -> bool {
        if !a_struct.is_sized() {
            return false;
        }
        
        if a_struct.count_fields() != 7 {
            return false;
        }
        
        if a_struct.is_packed() {
            return false;
        }
            
        if a_struct.is_opaque() {
            return false;
        }
        
        if a_struct.get_name() != Some(&*CString::new("evm.txctx").unwrap()) {
            return false;
        }

        let field1 = a_struct.get_field_type_at_index(0).unwrap();
        if !field1.is_int_type() {
            return false;
        }
        
        if field1.into_int_type().get_bit_width() != 256 {
            return false;
        }
        
        let field2 = a_struct.get_field_type_at_index(1).unwrap();
        if !field2.is_array_type() {
            return false;
        }
        
        let array_type2 = field2.into_array_type();
        if array_type2.len() != 20 {
            return false;
        }
            
        if !array_type2.get_element_type().is_int_type() {
            return false;
        }
            
        if array_type2.get_element_type().into_int_type().get_bit_width() != 8 {
            return false
        }

        let field3 = a_struct.get_field_type_at_index(2).unwrap();
        if !field3.is_array_type() {
            return false;
        }
        
        let array_type3 = field3.into_array_type();
        if array_type3.len() != 20 {
            return false;
        }
            
        if !array_type3.get_element_type().is_int_type() {
            return false;
        }
            
        if array_type3.get_element_type().into_int_type().get_bit_width() != 8 {
            return false
        }

        
        let field4 = a_struct.get_field_type_at_index(3).unwrap();
        if !field4.is_int_type() {
            return false;
        }
        
        if field4.into_int_type().get_bit_width() != 64 {
            return false;
        }
 
        let field5 = a_struct.get_field_type_at_index(4).unwrap();
        if !field5.is_int_type() {
            return false;
        }
                
        if field5.into_int_type().get_bit_width() != 64 {
            return false;
        }
 
        let field6 = a_struct.get_field_type_at_index(5).unwrap();
        if !field6.is_int_type() {
            return false;
        }
        
        if field6.into_int_type().get_bit_width() != 64 {
            return false;
        }
 
        let field7 = a_struct.get_field_type_at_index(6).unwrap();
        if !field7.is_int_type() {
            return false;
        }
        
        if field7.into_int_type().get_bit_width() != 256 {
            return false;
        }

        true
    }
}


pub struct TransactionContextManager<'a> {
    m_tx_ctx_loaded : PointerValue,
    m_tx_ctx : PointerValue,
    m_load_tx_ctx_fn : FunctionValue,
    m_builder: &'a Builder,
    m_context: &'a Context,
}


impl<'a> TransactionContextManager<'a> {
    pub fn new(context: &'a Context, builder: &'a Builder, module: &Module) -> TransactionContextManager<'a> {
        let bool_t = context.bool_type();
        let tx_loaded = builder.build_alloca(bool_t, "txctx.loaded");
        builder.build_store(tx_loaded, bool_t.const_int(0, false));

        let env_data_singleton = EnvDataType::get_instance(&context);
        let tx_ctx_singleton = TransactionContextType::get_instance(&context);
        
        let tx_ctx_alloca = builder.build_alloca(tx_ctx_singleton.get_type(), "txctx");

        let tx_ctx_fn_t = context.void_type().fn_type(&[tx_ctx_alloca.get_type().into(),
                                                        env_data_singleton.get_ptr_type().into()], false);
        let tx_ctx_fn = module.add_function ("evm.get_tx_context", tx_ctx_fn_t, Some(External));

        let load_tx_ctx_fn_t = context.void_type().fn_type(&[tx_loaded.get_type().into(),
                                                             tx_ctx_alloca.get_type().into(),
                                                             env_data_singleton.get_ptr_type().into()],
                                                           false);
        let load_tx_ctx_fn = module.add_function ("loadTxCtx", load_tx_ctx_fn_t, Some(Private));
        
        load_tx_ctx_fn.set_call_conventions(LLVMFastCallConv as u32);

        let check_bb = context.append_basic_block(&load_tx_ctx_fn, "Check");
        let load_bb = context.append_basic_block(&load_tx_ctx_fn, "Load");
        let exit_bb = context.append_basic_block(&load_tx_ctx_fn, "Exit");

        let flag = load_tx_ctx_fn.get_nth_param(0).unwrap();
        flag.into_pointer_value().set_name("flag");

        let tx_ctx = load_tx_ctx_fn.get_nth_param(1).unwrap();
        tx_ctx.into_pointer_value().set_name("txctx");
        
        let env = load_tx_ctx_fn.get_nth_param(2).unwrap();
        env.into_pointer_value().set_name("env");

        let temp_builder = context.create_builder();
        temp_builder.position_at_end(&check_bb);

        let flag_value = temp_builder.build_load(flag.into_pointer_value(), "");
        temp_builder.build_conditional_branch(flag_value.into_int_value(), &exit_bb, &load_bb);

        temp_builder.position_at_end(&load_bb);
        temp_builder.build_store (flag.into_pointer_value(), bool_t.const_int(1,false));
        temp_builder.build_call (tx_ctx_fn, &[tx_ctx.into(), env.into()], "");
        temp_builder.build_unconditional_branch(&exit_bb);

        temp_builder.position_at_end(&exit_bb);
        temp_builder.build_return(None);
        
        TransactionContextManager {
            m_tx_ctx_loaded : tx_loaded,
            m_tx_ctx : tx_ctx_alloca,
            m_load_tx_ctx_fn : load_tx_ctx_fn,
            m_builder : builder,
            m_context : context
        }
    }

    pub fn get_tx_ctx_type(&self) -> & TransactionContextType {
        TransactionContextType::get_instance(self.m_context)
    }

    pub fn get_tx_ctx_loaded_ssa_var(&self) -> PointerValue {
        self.m_tx_ctx_loaded
    }

    pub fn get_tx_ctx_ssa_var(&self) -> PointerValue {
        self.m_tx_ctx
    }

    pub fn get_tx_ctx_fn_ssa_var(&self) -> FunctionValue {
        self.m_load_tx_ctx_fn
    }
}


#[cfg(test)]
mod tests {
    //use std::ffi::CString;
    use super::*;
    use inkwell::values::InstructionOpcode;
    use inkwell::values::BasicValue;
    
    #[test]
    fn test_tx_ctx_type() {
        let context = Context::create();
        let tx_ctx_type_singleton = TransactionContextType::get_instance(&context);
        let tx_ctx = tx_ctx_type_singleton.get_type();

        assert!(TransactionContextType::is_transaction_context_type (&tx_ctx));
    }

    #[test]
    fn test_load_txctx_fn_instructions() {
        use super::super::MainFuncCreator;
        let context = Context::create();
        let module = context.create_module("evm_module");
        let builder = context.create_builder();

        // Need to create main function before TransactionContextManager otherwise we will crash
        MainFuncCreator::new ("main", &context, &builder, &module);
        
        TransactionContextManager::new(&context, &builder, &module);
        let load_tx_ctx_fn_optional = module.get_function ("loadTxCtx");
        assert!(load_tx_ctx_fn_optional != None);

        let load_tx_ctx_fn = load_tx_ctx_fn_optional.unwrap();
        assert_eq!(load_tx_ctx_fn.get_call_conventions(), LLVMFastCallConv as u32);
        assert_eq!(load_tx_ctx_fn.count_params(), 3);
        assert_eq!(load_tx_ctx_fn.count_basic_blocks(), 3);

        let func_parm1_opt = load_tx_ctx_fn.get_nth_param(0);
        assert!(!func_parm1_opt.is_none());

        // Verify paramter 1 is pointer to int1
        let func_parm1_t = func_parm1_opt.unwrap().get_type();
        assert!(func_parm1_t.is_pointer_type());
        let funct_param1_elem_t = func_parm1_t.as_pointer_type().get_element_type();
        assert!(funct_param1_elem_t.is_int_type());
        assert_eq!(funct_param1_elem_t.as_int_type().get_bit_width(), 1);

        // Verify parameter 2 is pointer to TransactionContext
        let func_parm2_opt = load_tx_ctx_fn.get_nth_param(1);
        assert!(!func_parm2_opt.is_none());
        let func_parm2_t = func_parm2_opt.unwrap().get_type();
        assert!(func_parm2_t.is_pointer_type());
        let funct_param2_elem_t = func_parm2_t.as_pointer_type().get_element_type();
        assert!(funct_param2_elem_t.is_struct_type());

        let the_struct_type = funct_param2_elem_t.as_struct_type();
        assert!(TransactionContextType::is_transaction_context_type (&the_struct_type));

        // Verify parameter 3 is pointer to EnvDataType
        let func_parm3_opt = load_tx_ctx_fn.get_nth_param(2);
        assert!(!func_parm3_opt.is_none());
        let func_parm3_t = func_parm3_opt.unwrap().get_type();
        assert!(func_parm3_t.is_pointer_type());
        let funct_param3_elem_t = func_parm3_t.as_pointer_type().get_element_type();
        assert!(funct_param3_elem_t.is_struct_type());

        let the_struct_type2 = funct_param3_elem_t.as_struct_type();
        assert!(EnvDataType::is_env_data_type (&the_struct_type2));

        let check_block_optional = load_tx_ctx_fn.get_first_basic_block();
        assert!(check_block_optional != None);
        let check_block = check_block_optional.unwrap();
        assert_eq!(*check_block.get_name(), *CString::new("Check").unwrap());

        assert!(check_block.get_first_instruction() != None);
        let first_check_insn = check_block.get_first_instruction().unwrap();
        assert_eq!(first_check_insn.get_opcode(), InstructionOpcode::Load);
        assert_eq!(first_check_insn.get_num_operands(), 1);

        let load_operand0 = first_check_insn.get_operand(0).unwrap();
        assert!(load_operand0.is_pointer_value());

        let load_operand0_ptr_elt_t = load_operand0.into_pointer_value().get_type().get_element_type();

        assert!(load_operand0_ptr_elt_t.is_int_type());
        assert_eq!(load_operand0_ptr_elt_t.as_int_type().get_bit_width(), 1);

        assert!(first_check_insn.get_next_instruction() != None);
        let second_check_insn = first_check_insn.get_next_instruction().unwrap();
        assert_eq!(second_check_insn.get_opcode(), InstructionOpcode::Br);
        // Condition branch has two operand, unconditional has 2
        assert_eq!(second_check_insn.get_num_operands(), 3);

        let br_operand0 = second_check_insn.get_operand(0).unwrap();
        let insn_setting_br_operand0 = br_operand0.as_instruction_value().unwrap();
        assert_eq!(insn_setting_br_operand0, first_check_insn);

        // Check for end of basic block
        assert!(second_check_insn.get_next_instruction() == None);

        let load_block_optional = check_block.get_next_basic_block();
        assert!(load_block_optional != None);
        let load_block = load_block_optional.unwrap();
        assert_eq!(*load_block.get_name(), *CString::new("Load").unwrap());
        let first_load_bb_insn = load_block.get_first_instruction().unwrap();
        assert_eq!(first_load_bb_insn.get_opcode(), InstructionOpcode::Store);
        assert_eq!(first_load_bb_insn.get_num_operands(), 2);

        let store_operand0 = first_load_bb_insn.get_operand(0).unwrap();
        assert!(store_operand0.is_int_value());
        let store_operand0_value = store_operand0.into_int_value();
        assert_eq!(store_operand0_value, context.bool_type().const_int(1, false));

        let store_operand1 = first_load_bb_insn.get_operand(1).unwrap();
        assert!(store_operand1.is_pointer_value());
        let store_operand1_ptr_elt_t = store_operand1.into_pointer_value().get_type().get_element_type();

        assert!(store_operand1_ptr_elt_t.is_int_type());
        assert_eq!(store_operand1_ptr_elt_t.as_int_type().get_bit_width(), 1);

        assert!(first_load_bb_insn.get_next_instruction() != None);
        let second_load_bb_insn = first_load_bb_insn.get_next_instruction().unwrap();
        assert_eq!(second_load_bb_insn.get_opcode(), InstructionOpcode::Call);
        assert_eq!(second_load_bb_insn.get_num_operands(), 3);

        let call_operand0 = second_load_bb_insn.get_operand(0).unwrap();
        assert!(call_operand0.is_pointer_value());   // should be pointer to transaction context

        let call_operand0_elem_t = call_operand0.as_pointer_value().get_type().get_element_type();
        assert!(call_operand0_elem_t.is_struct_type());

        let the_struct_type = call_operand0_elem_t.as_struct_type();
        assert!(TransactionContextType::is_transaction_context_type (&the_struct_type));

        let call_operand1 = second_load_bb_insn.get_operand(1).unwrap();
        assert!(call_operand1.is_pointer_value());   // should be pointer to environment

        let call_operand1_elem_t = call_operand1.as_pointer_value().get_type().get_element_type();
        assert!(call_operand1_elem_t.is_struct_type());

        let the_struct_type2 = call_operand1_elem_t.as_struct_type();
        assert!(EnvDataType::is_env_data_type (&the_struct_type2));

        assert!(second_load_bb_insn.get_next_instruction() != None);
        let third_load_bb_insn = second_load_bb_insn.get_next_instruction().unwrap();
        assert_eq!(third_load_bb_insn.get_opcode(), InstructionOpcode::Br);
        assert_eq!(third_load_bb_insn.get_num_operands(), 1);

        assert!(third_load_bb_insn.get_next_instruction() == None);

        let exit_block_optional = load_block.get_next_basic_block();
        assert!(exit_block_optional != None);
        let exit_block = exit_block_optional.unwrap();
        assert_eq!(*exit_block.get_name(), *CString::new("Exit").unwrap());
        let first_exit_bb_insn = exit_block.get_first_instruction().unwrap();
        assert_eq!(first_exit_bb_insn.get_opcode(), InstructionOpcode::Return);

        assert!(first_exit_bb_insn.get_next_instruction() == None);

    }

    #[test]
    fn test_transaction_context_manager() {
        use super::super::MainFuncCreator;
        let context = Context::create();
        let module = context.create_module("my_module");
        let builder = context.create_builder();

        // Need to create main function before TransactionConextManager otherwise we will crash
        MainFuncCreator::new ("main", &context, &builder, &module);
        
        TransactionContextManager::new(&context, &builder, &module);
        let main_fn_optional = module.get_function ("main");
        assert!(main_fn_optional != None);

        let main_fn = main_fn_optional.unwrap();
        assert!(main_fn.count_params() == 1);

        assert!(main_fn.get_first_basic_block() != None);
        let entry_block = main_fn.get_first_basic_block().unwrap();
        assert_eq!(*entry_block.get_name(), *CString::new("Entry").unwrap());

        assert!(entry_block.get_first_instruction() != None);
        let entry_bb_first_insn = entry_block.get_first_instruction().unwrap();
        assert_eq!(entry_bb_first_insn.get_opcode(), InstructionOpcode::Alloca);
        assert_eq!(entry_bb_first_insn.get_num_operands(), 1);

        let mut alloca_operand0 = entry_bb_first_insn.get_operand(0).unwrap();
        assert!(alloca_operand0.is_int_value());

        let alloca_arg_t = context.i32_type();

        // Operand 0 of alloca is a '1', meaning reserve space for 1 byte
        assert_eq!(alloca_operand0.into_int_value(), alloca_arg_t.const_int(1, false));

        assert!(entry_bb_first_insn.get_next_instruction() != None);
        let entry_bb_second_insn = entry_bb_first_insn.get_next_instruction().unwrap();
        assert_eq!(entry_bb_second_insn.get_num_operands(), 2);

        assert_eq!(entry_bb_second_insn.get_opcode(), InstructionOpcode::Store);

        let store_operand0 = entry_bb_second_insn.get_operand(0).unwrap();
        assert!(store_operand0.is_int_value());
        let store_operand0_value = store_operand0.into_int_value();
        assert_eq!(store_operand0_value, context.bool_type().const_int(0, false));

        let store_operand1 = entry_bb_second_insn.get_operand(1).unwrap();
        assert!(store_operand1.is_pointer_value());
        let store_operand1_ptr_elt_t = store_operand1.into_pointer_value().get_type().get_element_type();

        assert!(store_operand1_ptr_elt_t.is_int_type());
        assert_eq!(store_operand1_ptr_elt_t.as_int_type().get_bit_width(), 1);

        assert!(entry_bb_second_insn.get_next_instruction() != None);
        let entry_bb_third_insn = entry_bb_second_insn.get_next_instruction().unwrap();
        assert_eq!(entry_bb_third_insn.get_num_operands(), 1);
        assert_eq!(entry_bb_third_insn.get_opcode(), InstructionOpcode::Alloca);

        alloca_operand0 = entry_bb_third_insn.get_operand(0).unwrap();
        assert!(alloca_operand0.is_int_value());

        let alloca_arg_t = context.i32_type();

        // TODO: Figure our why this is failing
        assert_eq!(alloca_operand0.into_int_value(), alloca_arg_t.const_int(128, false));
    }


    #[test]
    fn test_get_tx_ctx_type() {
        use super::super::MainFuncCreator;
        let context = Context::create();
        let module = context.create_module("my_module");
        let builder = context.create_builder();

        // Need to create main function before TransactionConextManager otherwise we will crash
        MainFuncCreator::new ("main", &context, &builder, &module);
        let manager = TransactionContextManager::new(&context, &builder, &module);

        let tx_type = manager.get_tx_ctx_type().get_type();
        assert!(TransactionContextType::is_transaction_context_type (&tx_type));
    }
    
}
