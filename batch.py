import subprocess

for seed in range(100):
    subprocess.run(["python", "run.py", str(seed)])
