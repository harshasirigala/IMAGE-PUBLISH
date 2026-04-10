import cv2
import time
import os

output_path = r"C:\Users\harsh\Downloads\snapshots"
os.makedirs(output_path, exist_ok=True)

cap = cv2.VideoCapture(0)
cap.set(cv2.CAP_PROP_FRAME_WIDTH, 1280)
cap.set(cv2.CAP_PROP_FRAME_HEIGHT, 720)

print("Webcam opened. Warming up...")
time.sleep(3)  # longer warmup

for i in range(10):  # try 10 times
    ret, frame = cap.read()
    if ret:
        filename = os.path.join(output_path, "snapshot.jpg")
        cv2.imwrite(filename, frame, [cv2.IMWRITE_JPEG_QUALITY, 85])
        print(f"Saved to {filename}")
        break
    print(f"Attempt {i+1} failed, retrying...")
    time.sleep(1)
else:
    print("Failed to capture after 10 attempts")

cap.release()