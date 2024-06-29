import os
import sys
import requests
import time


def upload_file(secret_key: str, ip_address: str, folder_to_pack: str) -> bool:
    url = f"http://{ip_address}:2222/upload/zip"
    headers = {"Authorization": secret_key, "Content-Type": "multipart/form-data"}
    zip_file = "test.zip"
    zip_result = os.system(f"zip -r {zip_file} {folder_to_pack}")
    if zip_result != 0:
        print(f"Failed to zip {folder_to_pack}")
        return False

    with open(zip_file, "rb") as f:
        file_content = f.read()

    for attempt in range(3):
        files = {"file": file_content}
        response = requests.post(url, headers=headers, files=files)
        if response.status_code == 200:
            print("Upload succeeded.")
            return True

        time.sleep(1)

    return False


if __name__ == "__main__":
    if len(sys.argv) != 4:
        print("Usage: python3 upload.py <secret_key> <ip_address> <zip_file>")
        sys.exit(1)

    secret_key = sys.argv[1]
    ip_address = sys.argv[2]
    folder_to_pack = sys.argv[3]

    result = upload_file(secret_key, ip_address, folder_to_pack)
    if not result:
        print("Upload failed.")
        sys.exit(1)
