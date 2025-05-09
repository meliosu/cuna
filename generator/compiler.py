import os
import subprocess

class Compiler:
    def __init__(self):
        pass

    def compile(self, code: str, keep_tmps=False, debug=False):
        BUILD_DIR = "cuna-build"
        COMPILER = "gcc"
        OPT_LEVEL = 2

        if not os.path.exists(BUILD_DIR):
            os.mkdir(BUILD_DIR)
        
        with open(f"{BUILD_DIR}/model.c", "w+") as model:
            model.write(code)

        if debug:
            subprocess.run(
                f"clang-format {BUILD_DIR}/model.c",
                shell=True,
                check=True
            )

        subprocess.run(
            f"{COMPILER} -O{OPT_LEVEL} -o {BUILD_DIR}/model.o -c {BUILD_DIR}/model.c -I../c/", 
            shell=True,
            check=True
        )

        subprocess.run(
            f"{COMPILER} -o {BUILD_DIR}/libmodel.so {BUILD_DIR}/model.o -shared",
            shell=True,
            check=True
        )

        if not keep_tmps:
            os.remove(f"{BUILD_DIR}/model.c")
            os.remove(f"{BUILD_DIR}/model.o")
            os.remove(f"{BUILD_DIR}/libmodel.so")
            os.rmdir(f"{BUILD_DIR}")
