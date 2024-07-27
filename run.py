import subprocess
import sys
import ctypes


def is_admin():
    try:
        return ctypes.windll.shell32.IsUserAnAdmin()
    except:
        return False


def main():
    sandboxie_path = r"C:\Program Files\Sandboxie-Plus\Start.exe"
    box_name = "/box:LockdownBrowser"
    browser_executable = r"D:\Sandbox\LockdownBrowser\drive\C\Program Files (x86)\Respondus\LockDown Browser\LockDownBrowser.exe"
    rldb_string = "rldb:..."

    command = f'"{sandboxie_path}" {box_name} "{browser_executable}" "{rldb_string}"'

    if not is_admin():
        print(
            "This script requires administrative privileges. Please run as administrator."
        )
        ctypes.windll.shell32.ShellExecuteW(
            None, "runas", sys.executable, " ".join(sys.argv), None, 1
        )
    else:
        try:
            subprocess.call(command, shell=True)
            print("Command executed successfully.")
        except Exception as e:
            print(f"An error occurred: {e}")


if __name__ == "__main__":
    main()
