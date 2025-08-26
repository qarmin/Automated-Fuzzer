import os
import sys
import subprocess

file_names = sys.argv[1].split(",")

for file_name in file_names:
    os.mkdir(file_name)
    os.chdir(file_name)
    url = (
        f"https://github.com/qarmin/Automated-Fuzzer/releases/download/test/{file_name}"
    )
    subprocess.run(["wget", "-q", url])
    if file_name.endswith(".zip"):
        subprocess.run(["unzip", "-q", f"{file_name}"])
    elif file_name.endswith(".7z"):
        subprocess.run(["7z", "x", f"{file_name}"])
    else:
        raise Exception("Nieobsługiwany format pliku")
    os.remove(f"{file_name}")
    os.chdir("..")
