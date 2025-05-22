import os
import subprocess
import platform
import shutil

class Compiler:
    def __init__(self):
        pass

    def compile(
        self, 
        code: str, 
        debug: bool,
        keep_temp: bool,
        runtime: str,
        model_name: str,
        ucodes: str,
        output_dir = ".",
        build_dir = "cuna-build"
    ):
        # Determine compiler based on platform
        if platform.system() == "Windows":
            COMPILER = "nvcc" 
        else:
            COMPILER = "gcc"
        
        OPT_LEVEL = 2

        if not os.path.exists(build_dir):
            os.makedirs(build_dir, exist_ok=True)

        if not os.path.exists(output_dir):
            os.makedirs(output_dir, exist_ok=True)
        
        # Format code with clang-format if debug mode is enabled
        if debug:
            try:
                clang_process = subprocess.run(
                    ["clang-format"],
                    input=code.encode(),
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    check=True
                )
                
                code = clang_process.stdout.decode()
                print(code)
            except subprocess.CalledProcessError as e:
                print(f"Warning: clang-format failed: {e}")
        
        model_path = os.path.join(build_dir, "model.cu")
        with open(model_path, "w+") as model:
            model.write(code)

        if platform.system() == "Windows":
            model_obj = os.path.join(build_dir, "model.obj")
        else:
            model_obj = os.path.join(build_dir, "model.o")

        include_path = os.path.normpath(os.path.join(os.path.dirname(__file__), "..", "c"))
        
        subprocess.run(
            f"{COMPILER} -O{OPT_LEVEL} -o {model_obj} -c {model_path} -I{include_path}", 
            shell=True,
            check=True
        )

        output_exe = os.path.join(output_dir, f"{model_name}")
        if platform.system() == "Windows":
            output_exe += ".exe"
            
        runtime_dir = os.path.dirname(runtime)
        runtime_lib = os.path.basename(runtime)
        
        subprocess.run(
            f"{COMPILER} -o {output_exe} {model_obj} {ucodes} -L{runtime_dir} -l{runtime_lib}"
            " -l ws2_32 -l userenv -l ntdll -l kernel32 -l advapi32",
            shell=True,
            check=True
        )

        if keep_temp:
            return

        # Clean up
        if os.path.exists(model_path):
            os.remove(model_path)
            
        if os.path.exists(model_obj):
            os.remove(model_obj)
            
        if os.path.exists(build_dir) and os.path.isdir(build_dir):
            shutil.rmtree(build_dir)
