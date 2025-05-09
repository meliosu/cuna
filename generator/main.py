from generator import Generator
import json
import os
from subprocess import call
import sys

DEBUG = True

if __name__ == "__main__":
    with open("../examples/example.json", "r") as example:
        model = json.load(example)

    inputs = { "a", "b" }
    outputs = { "sum", "diff" }

    generator = Generator()
    code = generator.generate(model, inputs, outputs)

    if not os.path.exists("cuna-build"):
        os.mkdir("cuna-build")

    with open("cuna-build/model.c", "w+") as out:
        out.write(code)

    if DEBUG:
        call(["clang-format", "cuna-build/model.c"])