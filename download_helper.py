import os
import sys
import subprocess

# Pobierz listę elementów oddzielonych przecinkami jako argument wejściowy
file_names = sys.argv[1].split(',')

for file_name in file_names:
    os.mkdir(file_name)
    os.chdir(file_name)
    url = f"https://github.com/qarmin/Automated-Fuzzer/releases/download/test/{file_name}.zip"
    subprocess.run(["wget", url])
    subprocess.run(["unzip", f"{file_name}.zip"])
    os.remove(f"{file_name}.zip")
    os.chdir('..')