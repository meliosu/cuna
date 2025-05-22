use crate::ffi::{Model, Type};
use std::ffi::CStr;
use std::sync::{Arc, Mutex, RwLock};
use std::collections::{HashMap, HashSet};
use crossbeam::channel::{Sender, Receiver, unbounded};
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use rayon::ThreadPoolBuilder;
use std::time::Instant;
use crate::load_balancer::{LoadBalancer, DynamicLoadBalancer};

// Runtime state to track variables and operation status
struct RuntimeState {
    model: &'static Model,
    computed_variables: RwLock<HashMap<u32, Vec<u8>>>,
    ready_operations: Mutex<HashSet<u32>>,
    in_progress_operations: Mutex<HashSet<u32>>, 
    completed_operations: RwLock<HashSet<u32>>,
    work_channel: (Mutex<Sender<u32>>, Mutex<Receiver<u32>>),
    all_done: AtomicBool,
    load_balancer: Mutex<Box<dyn LoadBalancer>>,
}

// Runtime State
static RUNTIME: std::sync::OnceLock<Arc<RuntimeState>> = std::sync::OnceLock::new();

fn get_type_size(ty: &Type) -> usize {
    match ty {
        Type::I8 | Type::U8 => 1,
        Type::I16 | Type::U16 => 2,
        Type::I32 | Type::U32 | Type::F32 => 4,
        Type::I64 | Type::U64 | Type::F64 | Type::Pointer => 8,
        Type::Isize | Type::Usize => std::mem::size_of::<usize>(),
        Type::String => 8, 
    }
}

pub(crate) fn request(id: u32, var: *mut u8) {
    let runtime = RUNTIME.get().expect("Runtime not initialized");
    let variables = runtime.computed_variables.read().unwrap();
    
    if let Some(value) = variables.get(&id) {
        // Copy the variable value to the provided pointer
        unsafe {
            std::ptr::copy(value.as_ptr(), var, value.len());
        }
    } else {
        panic!("Variable with ID {} requested but not yet computed", id);
    }
}

pub(crate) fn submit(id: u32, var: *mut u8) {
    let runtime = RUNTIME.get().expect("Runtime not initialized");
    let var_info = &runtime.model.variables[id as usize];
    let size = get_type_size(&var_info.ty);
    
    // Copy the value from the provided pointer
    let mut value = vec![0u8; size];
    unsafe {
        std::ptr::copy(var, value.as_mut_ptr(), size);
    }
    
    // Store the computed value
    {
        let mut variables = runtime.computed_variables.write().unwrap();
        variables.insert(id, value);
    }
    
    // Check if any operations are now ready to run
    check_ready_operations();
}

fn check_ready_operations() {
    let runtime = RUNTIME.get().expect("Runtime not initialized");
    
    // Check each operation
    for op_id in 0..runtime.model.operations.len() {
        // Skip if already completed or already marked ready
        {
            let completed = runtime.completed_operations.read().unwrap();
            let mut ready = runtime.ready_operations.lock().unwrap();
            
            if completed.contains(&(op_id as u32)) || ready.contains(&(op_id as u32)) {
                continue;
            }
            
            // Check if all inputs are available
            let op = &runtime.model.operations[op_id];
            let computed_vars = runtime.computed_variables.read().unwrap();
            
            let all_inputs_available = op.inputs.iter()
                .all(|&input_id| computed_vars.contains_key(&input_id));
                
            if all_inputs_available {
                // Mark as ready and send to worker queue
                ready.insert(op_id as u32);
                let sender = runtime.work_channel.0.lock().unwrap();
                sender.send(op_id as u32).unwrap();
            }
        }
    }
}

fn worker_function() {
    let runtime = RUNTIME.get().expect("Runtime not initialized");
    let receiver = runtime.work_channel.1.lock().unwrap().clone();
    
    while !runtime.all_done.load(Ordering::Relaxed) {
        // Atomically get an operation using the load balancer and mark it as in-progress
        let op_id_option = {
            let mut ready_ops = runtime.ready_operations.lock().unwrap();
            let mut in_progress = runtime.in_progress_operations.lock().unwrap();
            let load_balancer = runtime.load_balancer.lock().unwrap();
            
            // Create a set of operations that are ready but not in progress
            let available_ops: HashSet<_> = ready_ops
                .difference(&in_progress)
                .copied()
                .collect();
            
            // Select the next operation
            let selected = load_balancer.select_operation(&available_ops, runtime.model);
            
            // If an operation was selected, mark it as in-progress and remove from ready queue
            if let Some(op_id) = selected {
                ready_ops.remove(&op_id);
                in_progress.insert(op_id);
            }
            
            selected
        };
        
        // If load balancer suggested an operation, use it; otherwise try the channel
        let op_id = match op_id_option {
            Some(id) => Some(id),
            None => match receiver.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(id) => {
                    // When getting from channel, also mark as in-progress atomically
                    let mut in_progress = runtime.in_progress_operations.lock().unwrap();
                    
                    // Double-check that this operation isn't already in progress
                    if !in_progress.contains(&id) {
                        in_progress.insert(id);
                        Some(id)
                    } else {
                        None
                    }
                },
                Err(_) => None,
            }
        };
        
        if let Some(op_id) = op_id {
            // Skip if already completed (double check)
            {
                let completed = runtime.completed_operations.read().unwrap();
                if completed.contains(&op_id) {
                    // Remove from in-progress if it was already completed
                    let mut in_progress = runtime.in_progress_operations.lock().unwrap();
                    in_progress.remove(&op_id);
                    continue;
                }
            }
            
            // Execute the operation with timing
            let op = &runtime.model.operations[op_id as usize];
            let start_time = Instant::now();
            unsafe { (op.function)() };
            let execution_time = start_time.elapsed();
            
            // Update load balancer statistics
            {
                let mut load_balancer = runtime.load_balancer.lock().unwrap();
                load_balancer.update_statistics(op_id, op.device, execution_time);
            }
            
            // Mark as completed and remove from in-progress
            {
                let mut completed = runtime.completed_operations.write().unwrap();
                completed.insert(op_id);
                
                let mut in_progress = runtime.in_progress_operations.lock().unwrap();
                in_progress.remove(&op_id);
            }
            
            // Check if any new operations are ready
            check_ready_operations();
            
            // Check if all outputs are computed
            let all_outputs_computed = runtime.model.outputs.iter()
                .all(|&output_id| {
                    let vars = runtime.computed_variables.read().unwrap();
                    vars.contains_key(&output_id)
                });
                
            if all_outputs_computed {
                runtime.all_done.store(true, Ordering::Relaxed);
            }
        } else if runtime.all_done.load(Ordering::Relaxed) {
            break;
        }
    }
}

fn parse_input_args() {
    let runtime = RUNTIME.get().expect("Runtime not initialized");
    let args: Vec<String> = env::args().collect();
    
    // Check if we have the right number of args
    if args.len() - 1 != runtime.model.inputs.len() {
        panic!("Expected {} input arguments, got {}", runtime.model.inputs.len(), args.len() - 1);
    }
    
    // Parse each input argument
    for (i, &input_id) in runtime.model.inputs.iter().enumerate() {
        let var_info = &runtime.model.variables[input_id as usize];
        let arg = &args[i + 1];
        
        // Parse based on variable type
        let value = match var_info.ty {
            Type::I8 => {
                let val = arg.parse::<i8>().expect("Failed to parse i8 input");
                val.to_ne_bytes().to_vec()
            },
            Type::U8 => {
                let val = arg.parse::<u8>().expect("Failed to parse u8 input");
                val.to_ne_bytes().to_vec()
            },
            Type::I16 => {
                let val = arg.parse::<i16>().expect("Failed to parse i16 input");
                val.to_ne_bytes().to_vec()
            },
            Type::U16 => {
                let val = arg.parse::<u16>().expect("Failed to parse u16 input");
                val.to_ne_bytes().to_vec()
            },
            Type::I32 => {
                let val = arg.parse::<i32>().expect("Failed to parse i32 input");
                val.to_ne_bytes().to_vec()
            },
            Type::U32 => {
                let val = arg.parse::<u32>().expect("Failed to parse u32 input");
                val.to_ne_bytes().to_vec()
            },
            Type::I64 => {
                let val = arg.parse::<i64>().expect("Failed to parse i64 input");
                val.to_ne_bytes().to_vec()
            },
            Type::U64 => {
                let val = arg.parse::<u64>().expect("Failed to parse u64 input");
                val.to_ne_bytes().to_vec()
            },
            Type::Isize => {
                let val = arg.parse::<isize>().expect("Failed to parse isize input");
                val.to_ne_bytes().to_vec()
            },
            Type::Usize => {
                let val = arg.parse::<usize>().expect("Failed to parse usize input");
                val.to_ne_bytes().to_vec()
            },
            Type::F32 => {
                let val = arg.parse::<f32>().expect("Failed to parse f32 input");
                val.to_ne_bytes().to_vec()
            },
            Type::F64 => {
                let val = arg.parse::<f64>().expect("Failed to parse f64 input");
                val.to_ne_bytes().to_vec()
            },
            Type::String => {
                let c_string = std::ffi::CString::new(arg.clone()).expect("Failed to create CString");

                // Leak the string to ensure it lives for the duration of the program
                let ptr = c_string.as_ptr() as usize;
                std::mem::forget(c_string);
                ptr.to_ne_bytes().to_vec()
            },
            _ => panic!("Unsupported type"),
        };
        
        let mut variables = runtime.computed_variables.write().unwrap();
        variables.insert(input_id, value);
    }
}

fn print_output_variables() {
    let runtime = RUNTIME.get().expect("Runtime not initialized");
    let vars = runtime.computed_variables.read().unwrap();
    
    for &output_id in runtime.model.outputs {
        let var_info = &runtime.model.variables[output_id as usize];
        
        if let Some(value) = vars.get(&output_id) {
            // Print based on variable type
            match var_info.ty {
                Type::I8 => {
                    let val = i8::from_ne_bytes([value[0]]);
                    println!("{}", val);
                },
                Type::U8 => {
                    let val = u8::from_ne_bytes([value[0]]);
                    println!("{}", val);
                },
                Type::I16 => {
                    let val = i16::from_ne_bytes([value[0], value[1]]);
                    println!("{}", val);
                },
                Type::U16 => {
                    let val = u16::from_ne_bytes([value[0], value[1]]);
                    println!("{}", val);
                },
                Type::I32 => {
                    let val = i32::from_ne_bytes([value[0], value[1], value[2], value[3]]);
                    println!("{}", val);
                },
                Type::U32 => {
                    let val = u32::from_ne_bytes([value[0], value[1], value[2], value[3]]);
                    println!("{}", val);
                },
                Type::I64 => {
                    let val = i64::from_ne_bytes([value[0], value[1], value[2], value[3], 
                                                 value[4], value[5], value[6], value[7]]);
                    println!("{}", val);
                },
                Type::U64 => {
                    let val = u64::from_ne_bytes([value[0], value[1], value[2], value[3], 
                                                 value[4], value[5], value[6], value[7]]);
                    println!("{}", val);
                },
                Type::Isize => {
                        let val = isize::from_ne_bytes([value[0], value[1], value[2], value[3], 
                                                      value[4], value[5], value[6], value[7]]);
                        println!("{}", val);
                },
                Type::Usize => {
                        let val = usize::from_ne_bytes([value[0], value[1], value[2], value[3], 
                                                      value[4], value[5], value[6], value[7]]);
                        println!("{}", val);
                },
                Type::F32 => {
                    let val = f32::from_ne_bytes([value[0], value[1], value[2], value[3]]);
                    println!("{}", val);
                },
                Type::F64 => {
                    let val = f64::from_ne_bytes([value[0], value[1], value[2], value[3], 
                                                 value[4], value[5], value[6], value[7]]);
                    println!("{}", val);
                },
                Type::String => {
                    let ptr_bytes = [value[0], value[1], value[2], value[3], 
                                     value[4], value[5], value[6], value[7]];
                    let ptr = usize::from_ne_bytes(ptr_bytes);
                    let c_string_ptr = ptr as *const i8;
                    let c_string = unsafe { CStr::from_ptr(c_string_ptr) };
                    
                    if !c_string_ptr.is_null() {
                        match c_string.to_str() {
                            Ok(s) => println!("{}", s),
                            Err(_) => println!("<invalid utf8 string>"),
                        }
                    } else {
                        println!("<null string>");
                    }
                },
                _ => panic!("Unsupported type"),
            }
        } else {
            println!("Output {} not computed", output_id);
        }
    }
}

pub(crate) fn launch(model: &'static Model) {
    // Create a global runtime state
    let (sender, receiver) = unbounded();
    let runtime = Arc::new(RuntimeState {
        model,
        computed_variables: RwLock::new(HashMap::new()),
        ready_operations: Mutex::new(HashSet::new()),
        in_progress_operations: Mutex::new(HashSet::new()), // Initialize the in-progress set
        completed_operations: RwLock::new(HashSet::new()),
        work_channel: (Mutex::new(sender), Mutex::new(receiver)),
        all_done: AtomicBool::new(false),
        load_balancer: Mutex::new(Box::new(DynamicLoadBalancer::new())),
    });
    
    // Initialize the global runtime
    RUNTIME.get_or_init(|| runtime);
    
    // Parse input arguments
    parse_input_args();
    
    // Check which operations are initially ready
    check_ready_operations();
    
    // Create worker threads with rayon
    let thread_count = rayon::current_num_threads();
    let pool = ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .build()
        .unwrap();
        
    pool.install(|| {
        rayon::scope(|s| {
            for _ in 0..thread_count {
                s.spawn(|_| {
                    worker_function();
                });
            }
        });
    });
    
    // Print output variables
    print_output_variables();
}