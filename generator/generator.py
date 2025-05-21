class Generator:
    out: str
    model: dict[any, any]
    inputs: set[str]
    outputs: set[str]

    def __init__(self):
        self.out = ""
        
    def generate(self, model: dict[any, any], inputs: set[str], outputs: set[str]) -> str:
        self.model = model
        self.inputs = inputs
        self.outputs = outputs

        self._gen_runtime_incldue()
        self._gen_ids()
        self._gen_operation_decls()
        self._gen_variables_and_operations_structs()
        self._gen_inputs_outputs()
        self._gen_model_definition()
        self._gen_module_decls()
        self._gen_operations()
        self._gen_main()

        return self.out

    def _gen_main(self):
        self.out += "int main(){ launch(&model); }"
    
    def _gen_runtime_incldue(self):
        self.out += "#include \"runtime.h\"\n"

    def _gen_ids(self):
        variable_ids = []
        operation_ids = []

        for name, _ in self.model["variables"].items():
            variable_ids.append(f"ID_{name}")

        for name, _ in self.model["operations"].items():
            operation_ids.append(f"ID_{name}")

        self.out += f"enum {{{','.join(variable_ids)}}};"
        self.out += f"enum {{{','.join(operation_ids)}}};"

    def _gen_variables_and_operations_structs(self):
        producers = {}
        consumers = {}
        inputs = {}
        outputs = {}

        for var_name, variable in self.model["variables"].items():
            for op_name, operation in self.model["operations"].items():
                if var_name in operation["outputs"]:
                    if var_name not in producers:
                        producers[var_name] = []

                    producers[var_name].append(op_name)
                
                elif var_name in operation["inputs"]:
                    if var_name not in consumers:
                        consumers[var_name] = []

                    consumers[var_name].append(op_name)

        for op_name, operation in self.model["operations"].items():
            if len(operation["inputs"]) > 0:
                inputs[op_name] = operation["inputs"]

            if len(operation["outputs"]) > 0:
                outputs[op_name] = operation["outputs"]

        for name, variable in self.model["variables"].items():
            if name in producers:
                self.out += f"uint32_t {name}_producers[{len(producers[name])}] = {{{','.join(map(lambda x: f'ID_{x}', producers[name]))}}};"

            if name in consumers:
                self.out += f"uint32_t {name}_consumers[{len(consumers[name])}] = {{{','.join(map(lambda x: f'ID_{x}', consumers[name]))}}};"

        for name, operation in self.model["operations"].items():
            if name in inputs:
                self.out += f"uint32_t {name}_inputs[{len(inputs[name])}] = {{{','.join(map(lambda x: f'ID_{x}', inputs[name]))}}};"

            if name in outputs:
                self.out += f"uint32_t {name}_outputs[{len(outputs[name])}] = {{{','.join(map(lambda x: f'ID_{x}', outputs[name]))}}};"

        self.out += f"Variable variables[] = {{"

        for name, variable in self.model["variables"].items():
            var_producers = producers[name] if name in producers else None
            var_consumers = consumers[name] if name in consumers else None

            self.out += f"{{"

            if var_producers is not None:
                self.out += f"{name}_producers,{len(var_producers)},"
            else:
                self.out += f"NULL,0,"

            if var_consumers is not None:
                self.out += f"{name}_consumers,{len(var_consumers)},"
            else:
                self.out += f"NULL,0,"

            self.out += variable["type"].capitalize()

            self.out += f"}},"

        self.out += f"}};"

        self.out += f"Operation operations[] = {{"

        for name, operation in self.model["operations"].items():
            op_inputs = inputs[name] if name in inputs else None
            op_outputs = outputs[name] if name in outputs else None

            self.out += f"{{"

            if op_inputs is not None:
                self.out += f"{name}_inputs,{len(op_inputs)},"
            else:
                self.out += f"NULL,0,"

            if op_outputs is not None:
                self.out += f"{name}_outputs,{len(op_outputs)},"
            else:
                self.out += f"NULL,0,"

            self.out += f"op_{name},"

            device = self.model["modules"][operation["module"]["name"]]["device"]

            if device == "host":
                self.out += "Host"
            else:
                self.out += "Cuda"

            self.out += f"}},"

        self.out += f"}};"

    def _gen_operation_decls(self):
        for name, operation in self.model["operations"].items():
            self.out += f"void op_{name}();"

    def _gen_operations(self):
        for name, operation in self.model["operations"].items():
            self.out += f"void op_{name}(){{"
            self.out += f"struct {{"

            for input in operation["inputs"]:
                ty = self.model["variables"][input]["type"]
                self.out += f"{ty} {input};"

            for output in operation["outputs"]:
                ty = self.model["variables"][output]["type"]
                self.out += f"{ty} {output};"

            self.out += f"}} ctx;"

            for input in operation["inputs"]:
                self.out += f"request(ID_{input}, &ctx.{input});"

            module = self.model["modules"][operation["module"]["name"]]
            args = []

            for arg in operation["module"]["args"]:
                if arg in operation["inputs"]:
                    args.append(f"ctx.{arg}")
                else:
                    args.append(f"&ctx.{arg}")

            self.out += f"{module['function']}({','.join(args)});"

            for output in operation["outputs"]:
                self.out += f"submit(ID_{output}, &ctx.{output});"

            self.out += f"}}"

    def _gen_module_decls(self):
        self.out += '#ifdef __cplusplus\nextern "C"{\n#endif'

        for name, module in self.model["modules"].items():
            params = []

            for param in module["args"]:
                if param["kind"] == "input":
                    params.append(f"{param['type']}")
                else:
                    params.append(f"{param['type']}*")

            self.out += f"void {name}({','.join(params)});"

        self.out += "#ifdef __cplusplus\n}\n#endif"

    def _gen_inputs_outputs(self):
        inputs = []
        outputs = []

        for input in self.inputs:
            inputs.append(f"ID_{input}")

        for output in self.outputs:
            outputs.append(f"ID_{output}")

        self.out += f"uint32_t inputs[{len(inputs)}] = {{{','.join(inputs)}}};"
        self.out += f"uint32_t outputs[{len(outputs)}] = {{{','.join(outputs)}}};"

    def _gen_model_definition(self):
        self.out += \
        f"Model model = {{" \
        f"variables," \
        f"{len(self.model['variables'].items())}," \
        f"operations," \
        f"{len(self.model['operations'].items())}," \
        f"inputs," \
        f"{len(self.inputs)}," \
        f"outputs," \
        f"{len(self.outputs)}," \
        f"}};"
