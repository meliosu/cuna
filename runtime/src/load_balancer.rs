use crate::ffi::{Device, Model};
use std::collections::HashSet;
use std::time::{Duration, Instant};

/// Interface for load balancing strategies
pub trait LoadBalancer: Send + Sync {
    /// Select the next operation to execute based on current system load
    /// Note: The provided ready_operations set should only contain operations that are
    /// neither in-progress nor completed
    fn select_operation(&self, ready_operations: &HashSet<u32>, model: &Model) -> Option<u32>;
    
    /// Update device statistics after an operation completes
    fn update_statistics(&mut self, op_id: u32, device: Device, execution_time: Duration);
    
    /// Get the current load for a specific device
    #[allow(unused)]
    fn device_load(&self, device: Device) -> f32;
}

/// A load balancer that dynamically adjusts based on CPU/GPU execution times
pub struct DynamicLoadBalancer {
    cpu_load: f32,
    gpu_load: f32,
    cpu_operation_count: u32,
    gpu_operation_count: u32,
    decay_factor: f32, // Weight for new measurements vs history
    last_cpu_time: Option<Instant>,
    last_gpu_time: Option<Instant>,
}

impl DynamicLoadBalancer {
    pub fn new() -> Self {
        Self {
            cpu_load: 0.0,
            gpu_load: 0.0,
            cpu_operation_count: 0,
            gpu_operation_count: 0,
            decay_factor: 0.8, // 80% weight to new measurements
            last_cpu_time: None,
            last_gpu_time: None,
        }
    }
}

impl LoadBalancer for DynamicLoadBalancer {
    fn select_operation(&self, ready_operations: &HashSet<u32>, model: &Model) -> Option<u32> {
        if ready_operations.is_empty() {
            return None;
        }
        
        // Group operations by device type
        let mut cpu_ops = Vec::new();
        let mut gpu_ops = Vec::new();
        
        for &op_id in ready_operations {
            let op = &model.operations[op_id as usize];
            match op.device {
                Device::Host => cpu_ops.push(op_id),
                Device::Cuda => gpu_ops.push(op_id),
            }
        }
        
        // If one device has no ops, choose from the other
        if cpu_ops.is_empty() {
            return gpu_ops.first().copied();
        }
        if gpu_ops.is_empty() {
            return cpu_ops.first().copied();
        }
        
        // Choose based on current load - prefer the less loaded device
        if self.cpu_load <= self.gpu_load {
            // CPU is less loaded or equally loaded, prefer CPU operation
            cpu_ops.first().copied()
        } else {
            // GPU is less loaded, prefer GPU operation
            gpu_ops.first().copied()
        }
    }
    
    fn update_statistics(&mut self, _op_id: u32, device: Device, execution_time: Duration) {
        match device {
            Device::Host => {
                self.cpu_operation_count += 1;
                
                // Update CPU load using exponential moving average
                let new_load = execution_time.as_secs_f32();
                self.cpu_load = (1.0 - self.decay_factor) * self.cpu_load + self.decay_factor * new_load;
                self.last_cpu_time = Some(Instant::now());
            },
            Device::Cuda => {
                self.gpu_operation_count += 1;
                
                // Update GPU load using exponential moving average
                let new_load = execution_time.as_secs_f32();
                self.gpu_load = (1.0 - self.decay_factor) * self.gpu_load + self.decay_factor * new_load;
                self.last_gpu_time = Some(Instant::now());
            },
        }
    }
    
    fn device_load(&self, device: Device) -> f32 {
        match device {
            Device::Host => self.cpu_load,
            Device::Cuda => self.gpu_load,
        }
    }
}
