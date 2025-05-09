import json
import os
import sys

from subprocess import call

from generator import Generator
from compiler import Compiler

if __name__ == "__main__":
    with open("../examples/example.json", "r") as example:
        model = json.load(example)

    inputs = { "a", "b" }
    outputs = { "sum", "diff" }

    generator = Generator()
    code = generator.generate(model, inputs, outputs)

    compiler = Compiler()
    compiler.compile(code, keep_tmps=True, debug=True)
