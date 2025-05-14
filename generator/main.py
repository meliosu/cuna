import json
import argparse
import os

from generator import Generator
from compiler import Compiler

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Generate code from computational model and VW-task")
    parser.add_argument("--model", required=True, help="Path to computational model")
    parser.add_argument("--inputs", required=True, help="List of input variables")
    parser.add_argument("--outputs", required=True, help="List of output variables")
    parser.add_argument("--runtime", required=True, help="Path to runtime library")
    parser.add_argument("--debug", action="store_true", help="Dump generated code to stdout")
    parser.add_argument("--output-dir", default=".", help="Path to directory where the final executable will be saved")
    parser.add_argument("--build-dir", default="cuna-build", help="Path to temporary build directory")
    parser.add_argument("--ucodes", required=True, help="Path to ucodes library/object file")

    args = parser.parse_args()

    with open(args.model, "r") as model_file:
        model = json.load(model_file)

    model_name = os.path.basename(args.model).removesuffix(".json")
    
    # Parse inputs and outputs as lists of strings separated by commas
    input_vars = [input_var.strip() for input_var in args.inputs.split(",")]
    output_vars = [output_var.strip() for output_var in args.outputs.split(",")]

    generator = Generator()
    code = generator.generate(model, input_vars, output_vars)

    compiler = Compiler()
    compiler.compile(
        code,
        debug=args.debug,
        runtime=args.runtime,
        model_name=model_name,
        output_dir=args.output_dir,
        build_dir=args.build_dir,
        ucodes=args.ucodes
    )