import cv2
import time
import os
from datetime import datetime

output_path = "/Users/harshasirigala/snapshots"
os.makedirs(output_path, exist_ok=True)

cap = cv2.VideoCapture(0)
cap.set(cv2.CAP_PROP_FRAME_WIDTH, 1280)
cap.set(cv2.CAP_PROP_FRAME_HEIGHT, 720)

print("Webcam opened. Warming up...")
time.sleep(2)

for i in range(10):
    ret, frame = cap.read()
    if ret:
        timestamp = datetime.now().strftime("%Y%m%dT%H%M%S")
        filename = os.path.join(output_path, f"snapshot_{timestamp}.jpg")
        cv2.imwrite(filename, frame, [cv2.IMWRITE_JPEG_QUALITY, 85])
        print(f"Saved to {filename}")
        break
    print(f"Attempt {i+1} failed, retrying...")
    time.sleep(1)
else:
    print("Failed to capture after 10 attempts")

cap.release()
