import copy

class Pruner:
    def __init__(self):
        pass
    
    def prune(self, model: dict, inputs: set[str], outputs: set[str]) -> dict:
        # Clone the original model to avoid modifying it
        pruned_model = copy.deepcopy(model)
        
        # Mark all variables and operations that are needed to calculate outputs
        needed_variables = set(inputs) | set(outputs) 
        needed_operations = set()
        
        # Traverse backward from outputs to mark all needed components
        unprocessed_variables = list(outputs)
        while unprocessed_variables:
            var_name = unprocessed_variables.pop()
            
            # Find operations that produce this variable
            for op_name, operation in model["operations"].items():
                if var_name in operation["outputs"]:
                    if op_name not in needed_operations:
                        needed_operations.add(op_name)
                        
                        # Add all inputs of this operation to needed variables
                        for input_var in operation["inputs"]:
                            if input_var not in needed_variables:
                                needed_variables.add(input_var)
                                unprocessed_variables.append(input_var)
        
        # Remove unneeded operations
        operations_to_remove = set(model["operations"].keys()) - needed_operations
        for op_name in operations_to_remove:
            del pruned_model["operations"][op_name]
        
        # Remove unneeded variables
        variables_to_remove = set(model["variables"].keys()) - needed_variables
        for var_name in variables_to_remove:
            del pruned_model["variables"][var_name]
        
        # Only keep modules that are referenced by needed operations
        used_modules = set(pruned_model["operations"][op_name]["module"]["name"] 
                           for op_name in needed_operations)
        modules_to_remove = set(pruned_model["modules"].keys()) - used_modules
        for module_name in modules_to_remove:
            del pruned_model["modules"][module_name]
        
        return pruned_model
