#![allow(dead_code)]

use std::ffi::CString;
use singletonum::{Singleton, SingletonInit};
use inkwell::context::Context;
use inkwell::builder::Builder;
use inkwell::module::Module;
use inkwell::types::StructType;
use inkwell::types::PointerType;
use inkwell::values::PointerValue;
use inkwell::values::BasicValueEnum;
use inkwell::AddressSpace;
use super::super::memory::mem_representation::MemoryRepresentationType;
use super::env::EnvDataType;
use super::rt_data_type::RuntimeDataType;
use super::rt_data_type::RuntimeDataFieldToIndex;
use super::rt_data_type::RuntimeDataFieldToName;
use super::rt_data_type::RuntimeDataTypeFields::Gas;
use super::rt_data_type::RuntimeDataTypeFields::GasPrice;
use super::rt_data_type::RuntimeDataTypeFields::CallData;
use super::rt_data_type::RuntimeDataTypeFields::CallDataSize;
use super::rt_data_type::RuntimeDataTypeFields::Value;
use super::rt_data_type::RuntimeDataTypeFields::Code;
use super::rt_data_type::RuntimeDataTypeFields::CodeSize;
use super::rt_data_type::RuntimeDataTypeFields::Address;
use super::rt_data_type::RuntimeDataTypeFields::Sender;
use super::rt_data_type::RuntimeDataTypeFields::Depth;
use evmjit::ModuleLookup;

//use super::rt_data_type::NUM_RUNTIME_DATA_FIELDS;

#[derive(Debug, Singleton)]

// RuntimeType is the struct that the JIT will build to pass
// arguments from the VM to the contract at runtime

pub struct RuntimeType
{
    rt_type: StructType,
    rt_ptr_type: PointerType,
}

unsafe impl Sync for RuntimeType {}
unsafe impl Send for RuntimeType {}

impl SingletonInit for RuntimeType {
    type Init = Context;
    fn init(context: &Context) -> Self {
        let rt_data_ptr = RuntimeDataType::get_instance(&context).get_ptr_type();
        let env_ptr = EnvDataType::get_instance(&context).get_ptr_type();
        let mem_ptr = MemoryRepresentationType::get_instance(&context).get_type();

        let fields = [rt_data_ptr.into(), env_ptr.into(), mem_ptr.into()];
        let rt_struct = context.opaque_struct_type("Runtime");
        rt_struct.set_body(&fields, false);

        RuntimeType {
            rt_type : rt_struct,
            rt_ptr_type : rt_struct.ptr_type(AddressSpace::Generic)
        }
    }
}

impl RuntimeType {
    pub fn get_type(&self) -> StructType {
        self.rt_type
    }

    pub fn get_ptr_type(&self) -> PointerType {
        self.rt_ptr_type
    }

    pub fn is_runtime_type(a_struct: &StructType) -> bool {
        if !a_struct.is_sized() {
            return false;
        }

        if a_struct.count_fields() != 3 {
            return false;
        }

        if a_struct.is_packed() {
            return false;
        }

        if a_struct.is_opaque() {
            return false;
        }

        if a_struct.get_name() != Some(&*CString::new("Runtime").unwrap()) {
            return false;
        }

        let field1 = a_struct.get_field_type_at_index(0).unwrap();
        if !field1.is_pointer_type() {
            return false;
        }

        let field1_element_t = field1.as_pointer_type().get_element_type();
        if !field1_element_t.is_struct_type() {
            return false;
        }

        if !RuntimeDataType::is_rt_data_type(&field1_element_t.as_struct_type()) {
            return false;
        }

        let field2 = a_struct.get_field_type_at_index(1).unwrap();
        if !field2.is_pointer_type() {
            return false;
        }

        let field2_element_t = field2.as_pointer_type().get_element_type();
        if !field2_element_t.is_struct_type() {
            return false;
        }

        if !EnvDataType::is_env_data_type(&&field2_element_t.as_struct_type()) {
            return false;
        }

        let field3 = a_struct.get_field_type_at_index(2).unwrap();
        if !field3.is_struct_type() {
            return false;
        }

        let field3_element_t = field3.as_struct_type();
        if !MemoryRepresentationType::is_mem_representation_type(&field3_element_t) {
            return false;
        }

        true
    }
}

#[derive(Debug, Copy, Clone)]
pub struct RuntimeTypeManager<'a> {
    m_data_ptr: BasicValueEnum,
    m_mem_ptr: PointerValue,
    m_env_ptr: BasicValueEnum,
    m_rt_data_elts: [BasicValueEnum; 10],
    m_context:&'a Context,
    m_builder: &'a Builder,
    m_module: &'a Module,
}

impl<'a> RuntimeTypeManager<'a> {
    pub fn new(context: &'a Context, builder: &'a Builder, module: &'a Module) -> RuntimeTypeManager<'a> {

        let rt_ptr = RuntimeTypeManager::get_runtime_ptr_with_builder(&context, &builder);
        unsafe {
            let data_p = builder.build_load (builder.build_struct_gep(rt_ptr.into_pointer_value(), 0, ""), "dataPtr");
            assert_eq!(data_p.get_type().into_pointer_type(), RuntimeDataType::get_instance(context).get_ptr_type());

            let mem_p = builder.build_struct_gep(rt_ptr.into_pointer_value(), 2, "mem");

            assert_eq!(mem_p.get_type(), MemoryRepresentationType::get_instance(&context).get_ptr_type());

            let env_p = builder.build_load (builder.build_struct_gep(rt_ptr.into_pointer_value(), 1, ""), "env");
            assert_eq!(env_p.get_type().into_pointer_type(), EnvDataType::get_instance(&context).get_ptr_type());

            let data = builder.build_load (*data_p.as_pointer_value(), "data");

            RuntimeTypeManager {
                m_data_ptr: data_p,
                m_mem_ptr: mem_p,
                m_env_ptr: env_p,
                m_rt_data_elts: [builder.build_extract_value(data.into_struct_value(),
                                                             Gas.to_index() as u32,
                                                             Gas.to_name()),
                                 builder.build_extract_value(data.into_struct_value(),
                                                             GasPrice.to_index() as u32,
                                                             GasPrice.to_name()),
                                 builder.build_extract_value(data.into_struct_value(),
                                                             CallData.to_index() as u32,
                                                             CallData.to_name()),
                                 builder.build_extract_value(data.into_struct_value(),
                                                             CallDataSize.to_index() as u32,
                                                             CallDataSize.to_name()),
                                 builder.build_extract_value(data.into_struct_value(),
                                                             Value.to_index() as u32,
                                                             Value.to_name()),
                                 builder.build_extract_value(data.into_struct_value(),
                                                             Code.to_index() as u32,
                                                             Code.to_name()),
                                 builder.build_extract_value(data.into_struct_value(),
                                                             CodeSize.to_index() as u32,
                                                             CodeSize.to_name()),
                                 builder.build_extract_value(data.into_struct_value(),
                                                             Address.to_index() as u32,
                                                             Address.to_name()),
                                 builder.build_extract_value(data.into_struct_value(),
                                                             Sender.to_index() as u32,
                                                             Sender.to_name()),
                                 builder.build_extract_value(data.into_struct_value(),
                                                             Depth.to_index() as u32,
                                                             Depth.to_name())],
                m_context: context,
                m_builder: builder,
                m_module: module
            }
        }
    }

    pub fn get_runtime_ptr(&self) -> BasicValueEnum {
        RuntimeTypeManager::get_runtime_ptr_with_builder(self.m_context, self.m_builder)
    }

    fn get_runtime_ptr_with_builder(context: & Context, builder: & Builder) -> BasicValueEnum {
        // The parent of the first basic block is a function
        
        let bb = builder.get_insert_block();
        assert!(bb != None);
        
        let func = bb.unwrap().get_parent();
        assert!(func != None);
        let func_val = func.unwrap();
        
        // The first argument to a function is a pointer to the runtime
        assert!(func_val.count_params() > 0);

        let runtime_ptr = func_val.get_first_param().unwrap();
        assert_eq!(runtime_ptr.get_type().into_pointer_type(), RuntimeType::get_instance(context).get_ptr_type());

        runtime_ptr
    }

    pub fn get_env_ptr(self) -> BasicValueEnum {
        self.m_env_ptr
    }

    pub fn get_data_ptr(&self) -> BasicValueEnum {
        if self.m_module.get_main_function(self.m_builder) != None {
            self.m_data_ptr
        } else {
            // If we are not in the main function, get a pointer to the runtime type
            // which contains the data pointer

            let runtime_ptr = self.get_runtime_ptr();
            unsafe {
                let temp = self.m_builder.build_struct_gep(runtime_ptr.into_pointer_value(), 0, "");

                let data_p = self.m_builder.build_load(temp, "data");

                assert!(data_p.is_pointer_value());
                let elem_t = data_p.get_type().into_pointer_type().get_element_type();
                assert!(elem_t.is_struct_type());
                assert!(RuntimeDataType::is_rt_data_type(elem_t.as_struct_type()));
                data_p
            }
        }
    }

    pub fn get_mem_ptr(self) -> PointerValue {
        self.m_mem_ptr
    }

    pub fn get_address(self) -> BasicValueEnum {
        self.m_rt_data_elts[Address.to_index()]
    }

    pub fn get_sender(self) -> BasicValueEnum {
        self.m_rt_data_elts[Sender.to_index()]
    }

    pub fn get_value(self) -> BasicValueEnum {
        self.m_rt_data_elts[Value.to_index()]
    }

    pub fn get_depth(self) -> BasicValueEnum {
        self.m_rt_data_elts[Depth.to_index()]
    }

    pub fn get_gas(self) -> BasicValueEnum {
        self.m_rt_data_elts[Gas.to_index()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inkwell::values::InstructionOpcode;

    #[test]
    fn test_runtime_type() {
        let context = Context::create();
        let rt_type_singleton = RuntimeType::get_instance(&context);
        let rt_struct = rt_type_singleton.get_type();

        assert!(RuntimeType::is_runtime_type(&rt_struct));

        // Test that we have a pointer to RuntimeData

        let rt_struct_ptr = rt_type_singleton.get_ptr_type();
        assert!(rt_struct_ptr.get_element_type().is_struct_type());
        assert!(RuntimeType::is_runtime_type (rt_struct_ptr.get_element_type().as_struct_type()));
    }

    #[test]
    fn test_get_env_ptr() {
        use super::super::MainFuncCreator;
        let context = Context::create();
        let module = context.create_module("my_module");
        let builder = context.create_builder();

        // Need to create main function before RuntimeTypeManager otherwise we will crash
        MainFuncCreator::new ("main", &context, &builder, &module);

        let manager = RuntimeTypeManager::new (&context, &builder, &module);
        let env_ptr = manager.get_env_ptr();

        assert_eq!(env_ptr.get_type().into_pointer_type(), EnvDataType::get_instance(&context).get_ptr_type());
    }

    #[test]
    fn test_get_data_ptr() {
        use super::super::MainFuncCreator;
        let context = Context::create();
        let module = context.create_module("my_module");
        let builder = context.create_builder();

        // Need to create main function before RuntimeTypeManager otherwise we will crash
        MainFuncCreator::new ("main", &context, &builder, &module);

        let manager = RuntimeTypeManager::new (&context, &builder, &module);
        let data_ptr = manager.get_data_ptr();

        assert_eq!(data_ptr.get_type().into_pointer_type(), RuntimeDataType::get_instance(&context).get_ptr_type());
    }

    #[test]
    fn test_get_mem_ptr() {
        use super::super::MainFuncCreator;
        let context = Context::create();
        let module = context.create_module("my_module");
        let builder = context.create_builder();

        // Need to create main function before RuntimeTypeManager otherwise we will crash
        MainFuncCreator::new ("main", &context, &builder, &module);

        let manager = RuntimeTypeManager::new (&context, &builder, &module);
        let mem_ptr = manager.get_mem_ptr();

        assert_eq!(mem_ptr.get_type(), MemoryRepresentationType::get_instance(&context).get_ptr_type());
    }

    #[test]
    fn test_get_runtime_manager_new() {
        use super::super::MainFuncCreator;
        let context = Context::create();
        let module = context.create_module("my_module");
        let builder = context.create_builder();

        // Need to create main function before RuntimeTypeManager otherwise we will crash
        let main_func = MainFuncCreator::new ("main", &context, &builder, &module);
        RuntimeTypeManager::new (&context, &builder, &module);

        let entry_block = main_func.get_entry_bb();

        assert!(entry_block.get_first_instruction() != None);
        let first_insn = entry_block.get_first_instruction().unwrap();
        assert_eq!(first_insn.get_opcode(), InstructionOpcode::GetElementPtr);
        assert_eq!(first_insn.get_num_operands(), 3);

        let mut gep_operand0 = first_insn.get_operand(0).unwrap();
        assert!(gep_operand0.is_pointer_value());
        let mut gep_operand0_ptr_elt_t = gep_operand0.into_pointer_value().get_type().get_element_type();
        assert!(gep_operand0_ptr_elt_t.is_struct_type());
        let mut gep_operand0_type = gep_operand0_ptr_elt_t.into_struct_type();
        assert!(RuntimeType::is_runtime_type (&gep_operand0_type));

        let mut gep_operand1 = first_insn.get_operand(1).unwrap();
        assert!(gep_operand1.is_int_value());
        assert_eq!(gep_operand1.into_int_value(), context.i32_type().const_int(0, false));

        let mut gep_operand2 = first_insn.get_operand(2).unwrap();
        assert!(gep_operand2.is_int_value());
        assert_eq!(gep_operand2.into_int_value(), context.i32_type().const_int(0, false));

        assert!(first_insn.get_next_instruction() != None);
        let second_insn = first_insn.get_next_instruction().unwrap();
        assert_eq!(second_insn.get_opcode(), InstructionOpcode::Load);
        assert_eq!(second_insn.get_num_operands(), 1);
        let mut load_operand0 = second_insn.get_operand(0).unwrap();
        assert!(load_operand0.is_pointer_value());
        let mut load_operand0_ptr_elt_t = load_operand0.into_pointer_value().get_type().get_element_type();
        assert!(load_operand0_ptr_elt_t.is_pointer_type());

        let load_operand0_ptr_to_ptr = load_operand0_ptr_elt_t.as_pointer_type().get_element_type();
        assert!(load_operand0_ptr_to_ptr.is_struct_type());
        assert!(RuntimeDataType::is_rt_data_type (load_operand0_ptr_to_ptr.as_struct_type()));;
        assert!(second_insn.get_next_instruction() != None);

        let third_insn = second_insn.get_next_instruction().unwrap();
        assert_eq!(third_insn.get_opcode(), InstructionOpcode::GetElementPtr);

        gep_operand0 = third_insn.get_operand(0).unwrap();
        assert!(gep_operand0.is_pointer_value());
        gep_operand0_ptr_elt_t = gep_operand0.into_pointer_value().get_type().get_element_type();
        assert!(gep_operand0_ptr_elt_t.is_struct_type());
        gep_operand0_type = gep_operand0_ptr_elt_t.into_struct_type();
        assert!(RuntimeType::is_runtime_type (&gep_operand0_type));

        gep_operand1 = third_insn.get_operand(1).unwrap();
        assert!(gep_operand1.is_int_value());
        assert_eq!(gep_operand1.into_int_value(), context.i32_type().const_int(0, false));

        gep_operand2 = third_insn.get_operand(2).unwrap();
        assert!(gep_operand2.is_int_value());
        assert_eq!(gep_operand2.into_int_value(), context.i32_type().const_int(2, false));

        assert!(third_insn.get_next_instruction() != None);
        let fourth_insn = third_insn.get_next_instruction().unwrap();
        assert_eq!(fourth_insn.get_opcode(), InstructionOpcode::GetElementPtr);

        gep_operand0 = fourth_insn.get_operand(0).unwrap();
        assert!(gep_operand0.is_pointer_value());
        gep_operand0_ptr_elt_t = gep_operand0.into_pointer_value().get_type().get_element_type();
        assert!(gep_operand0_ptr_elt_t.is_struct_type());
        gep_operand0_type = gep_operand0_ptr_elt_t.into_struct_type();
        assert!(RuntimeType::is_runtime_type (&gep_operand0_type));

        gep_operand1 = fourth_insn.get_operand(1).unwrap();
        assert!(gep_operand1.is_int_value());
        assert_eq!(gep_operand1.into_int_value(), context.i32_type().const_int(0, false));

        gep_operand2 = fourth_insn.get_operand(2).unwrap();
        assert!(gep_operand2.is_int_value());
        assert_eq!(gep_operand2.into_int_value(), context.i32_type().const_int(1, false));

        assert!(fourth_insn.get_next_instruction() != None);
        let fifth_insn = fourth_insn.get_next_instruction().unwrap();
        assert_eq!(fifth_insn.get_opcode(), InstructionOpcode::Load);

        assert_eq!(fifth_insn.get_num_operands(), 1);
        load_operand0 = fifth_insn.get_operand(0).unwrap();
        assert!(load_operand0.is_pointer_value());
        load_operand0_ptr_elt_t = load_operand0.into_pointer_value().get_type().get_element_type();
        assert!(load_operand0_ptr_elt_t.is_pointer_type());

        let load_operand0_ptr_to_ptr = load_operand0_ptr_elt_t.as_pointer_type().get_element_type();
        assert!(load_operand0_ptr_to_ptr.is_struct_type());
        assert!(EnvDataType::is_env_data_type (load_operand0_ptr_to_ptr.as_struct_type()));;

        assert!(fifth_insn.get_next_instruction() != None);
        let sixth_insn = fifth_insn.get_next_instruction().unwrap();
        assert_eq!(sixth_insn.get_opcode(), InstructionOpcode::Load);

        assert_eq!(sixth_insn.get_num_operands(), 1);
        load_operand0 = sixth_insn.get_operand(0).unwrap();
        assert!(load_operand0.is_pointer_value());
        load_operand0_ptr_elt_t = load_operand0.into_pointer_value().get_type().get_element_type();
        assert!(load_operand0_ptr_elt_t.is_struct_type());

        assert!(RuntimeDataType::is_rt_data_type (load_operand0_ptr_elt_t.as_struct_type()));

        assert!(sixth_insn.get_next_instruction() != None);
        let seventh_insn = sixth_insn.get_next_instruction().unwrap();
        assert_eq!(seventh_insn.get_opcode(), InstructionOpcode::ExtractValue);


        assert!(seventh_insn.get_operand(0) != None);
        let mut extract_val_operand0 = seventh_insn.get_operand(0).unwrap();
        assert!(extract_val_operand0.is_struct_value());
        let extract_val_type = extract_val_operand0.into_struct_value().get_type();
        assert!(RuntimeDataType::is_rt_data_type (&extract_val_type));

        let eighth_insn = seventh_insn.get_next_instruction().unwrap();
        assert_eq!(eighth_insn.get_opcode(), InstructionOpcode::ExtractValue);

        extract_val_operand0 = eighth_insn.get_operand(0).unwrap();
        assert!(extract_val_operand0.is_struct_value());
        let extract_val_type = extract_val_operand0.into_struct_value().get_type();
        assert!(RuntimeDataType::is_rt_data_type (&extract_val_type));

        assert!(eighth_insn.get_next_instruction() != None);
        let ninth_insn = eighth_insn.get_next_instruction().unwrap();
        assert_eq!(ninth_insn.get_opcode(), InstructionOpcode::ExtractValue);

        extract_val_operand0 = ninth_insn.get_operand(0).unwrap();
        assert!(extract_val_operand0.is_struct_value());
        let extract_val_type = extract_val_operand0.into_struct_value().get_type();
        assert!(RuntimeDataType::is_rt_data_type (&extract_val_type));

        assert!(ninth_insn.get_next_instruction() != None);
        let tenth_insn = ninth_insn.get_next_instruction().unwrap();
        assert_eq!(tenth_insn.get_opcode(), InstructionOpcode::ExtractValue);

        extract_val_operand0 = tenth_insn.get_operand(0).unwrap();
        assert!(extract_val_operand0.is_struct_value());
        let extract_val_type = extract_val_operand0.into_struct_value().get_type();
        assert!(RuntimeDataType::is_rt_data_type (&extract_val_type));

        assert!(tenth_insn.get_next_instruction() != None);
        let eleventh_insn = tenth_insn.get_next_instruction().unwrap();
        assert_eq!(eleventh_insn.get_opcode(), InstructionOpcode::ExtractValue);

        extract_val_operand0 = eleventh_insn.get_operand(0).unwrap();
        assert!(extract_val_operand0.is_struct_value());
        let extract_val_type = extract_val_operand0.into_struct_value().get_type();
        assert!(RuntimeDataType::is_rt_data_type (&extract_val_type));

        assert!(eleventh_insn.get_next_instruction() != None);
        let twelfth_insn = eleventh_insn.get_next_instruction().unwrap();
        assert_eq!(twelfth_insn.get_opcode(), InstructionOpcode::ExtractValue);

        extract_val_operand0 = twelfth_insn.get_operand(0).unwrap();
        assert!(extract_val_operand0.is_struct_value());
        let extract_val_type = extract_val_operand0.into_struct_value().get_type();
        assert!(RuntimeDataType::is_rt_data_type (&extract_val_type));

        assert!(twelfth_insn.get_next_instruction() != None);
        let thirteenth_insn = twelfth_insn.get_next_instruction().unwrap();
        assert_eq!(thirteenth_insn.get_opcode(), InstructionOpcode::ExtractValue);

        extract_val_operand0 = thirteenth_insn.get_operand(0).unwrap();
        assert!(extract_val_operand0.is_struct_value());
        let extract_val_type = extract_val_operand0.into_struct_value().get_type();
        assert!(RuntimeDataType::is_rt_data_type (&extract_val_type));

        assert!(thirteenth_insn.get_next_instruction() != None);
        let fourteenth_insn = thirteenth_insn.get_next_instruction().unwrap();
        assert_eq!(fourteenth_insn.get_opcode(), InstructionOpcode::ExtractValue);

        extract_val_operand0 = fourteenth_insn.get_operand(0).unwrap();
        assert!(extract_val_operand0.is_struct_value());
        let extract_val_type = extract_val_operand0.into_struct_value().get_type();
        assert!(RuntimeDataType::is_rt_data_type (&extract_val_type));

        assert!(fourteenth_insn.get_next_instruction() != None);
        let fifteenth_insn = fourteenth_insn.get_next_instruction().unwrap();
        assert_eq!(fifteenth_insn.get_opcode(), InstructionOpcode::ExtractValue);

        extract_val_operand0 = fifteenth_insn.get_operand(0).unwrap();
        assert!(extract_val_operand0.is_struct_value());
        let extract_val_type = extract_val_operand0.into_struct_value().get_type();
        assert!(RuntimeDataType::is_rt_data_type (&extract_val_type));

        assert!(fifteenth_insn.get_next_instruction() != None);
        let sixteenth_insn = fifteenth_insn.get_next_instruction().unwrap();
        assert_eq!(sixteenth_insn.get_opcode(), InstructionOpcode::ExtractValue);

        extract_val_operand0 = sixteenth_insn.get_operand(0).unwrap();
        assert!(extract_val_operand0.is_struct_value());
        let extract_val_type = extract_val_operand0.into_struct_value().get_type();
        assert!(RuntimeDataType::is_rt_data_type (&extract_val_type));

        assert!(sixteenth_insn.get_next_instruction() == None);
    }
}
