import os
import subprocess

class Compiler:
    def __init__(self):
        pass

    def compile(
        self, 
        code: str, 
        debug: bool,
        runtime: str,
        model_name: str,
        ucodes: str,
        output_dir = ".",
        build_dir = "cuna-build"
    ):
        COMPILER = "gcc"
        OPT_LEVEL = 2

        if not os.path.exists(build_dir):
            os.mkdir(build_dir)
        
        with open(f"{build_dir}/model.c", "w+") as model:
            model.write(code)

        if debug:
            subprocess.run(
                f"clang-format {build_dir}/model.c",
                shell=True,
                check=True
            )

        subprocess.run(
            f"{COMPILER} -O{OPT_LEVEL} -o {build_dir}/model.o -c {build_dir}/model.c -I../c/", 
            shell=True,
            check=True
        )

        subprocess.run(
            f"{COMPILER} -o {output_dir}/{model_name} {build_dir}/model.o {runtime} {ucodes}",
            shell=True,
            check=True
        )

        os.remove(f"{build_dir}/model.c")
        os.remove(f"{build_dir}/model.o")
        os.rmdir(f"{build_dir}")
