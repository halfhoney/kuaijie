# 拍摄快截应用界面截图 v5：点击后校验画面变化，没变就微调坐标重试
import ctypes, time, os, sys, hashlib
from ctypes import wintypes
from PIL import ImageGrab

OUT = r"F:\远程桌面\kuaijie\website\assets"
os.makedirs(OUT, exist_ok=True)

user32 = ctypes.windll.user32
kernel32 = ctypes.windll.kernel32

def proc_is_kuaijie(hwnd):
    pid = wintypes.DWORD()
    user32.GetWindowThreadProcessId(hwnd, ctypes.byref(pid))
    h = kernel32.OpenProcess(0x0410, False, pid.value)
    if not h:
        return False
    path = ctypes.create_unicode_buffer(512)
    size = wintypes.DWORD(512)
    ok = False
    if kernel32.QueryFullProcessImageNameW(h, 0, path, ctypes.byref(size)):
        ok = path.value.lower().endswith("kuaijie.exe")
    kernel32.CloseHandle(h)
    return ok

main_hwnd = None
def enum_cb(hwnd, _):
    global main_hwnd
    buf = ctypes.create_unicode_buffer(256)
    user32.GetWindowTextW(hwnd, buf, 256)
    if buf.value == "快截" and proc_is_kuaijie(hwnd):
        main_hwnd = hwnd
    return True

user32.EnumWindows(ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)(enum_cb), 0)
if not main_hwnd:
    print("未找到快截主窗口"); sys.exit(1)
hwnd = main_hwnd

prev_fore = user32.GetForegroundWindow()
scale = user32.GetDpiForWindow(hwnd) / 96.0

cur_tid = kernel32.GetCurrentThreadId()
fore_tid = user32.GetWindowThreadProcessId(prev_fore, None)
user32.AttachThreadInput(cur_tid, fore_tid, True)
user32.ShowWindow(hwnd, 9)
time.sleep(0.2)
user32.ShowWindow(hwnd, 3)
user32.SetWindowPos(hwnd, -1, 0, 0, 0, 0, 0x0001 | 0x0002)
user32.SetForegroundWindow(hwnd)
user32.BringWindowToTop(hwnd)
user32.SetActiveWindow(hwnd)
user32.AttachThreadInput(cur_tid, fore_tid, False)
time.sleep(1.5)

rect = wintypes.RECT()
user32.GetWindowRect(hwnd, ctypes.byref(rect))
W, H = rect.right - rect.left, rect.bottom - rect.top
LH = H / scale
SW = user32.GetSystemMetrics(0)
SH = user32.GetSystemMetrics(1)
print(f"窗口 {W}x{H} 屏幕 {SW}x{SH} 缩放 {scale} 逻辑高 {LH:.0f}")

# 裁剪到屏幕实际可见范围
BX0, BY0 = max(rect.left, 0), max(rect.top, 0)
BX1, BY1 = min(rect.right, SW), min(rect.bottom, SH)

def grab_bytes():
    img = ImageGrab.grab(bbox=(BX0, BY0, BX1, BY1), all_screens=True)
    return img

def save(img, name):
    path = os.path.join(OUT, name)
    img.save(path)
    print(f"saved {name} {img.size} {os.path.getsize(path)}B")

def sig(img):
    return hashlib.md5(img.tobytes()).hexdigest()

def click_logical(lx, ly):
    x = int(rect.left + lx * scale)
    y = int(rect.top + ly * scale)
    user32.SetCursorPos(x, y)
    time.sleep(0.15)
    user32.mouse_event(0x0002, 0, 0, 0, 0)
    time.sleep(0.06)
    user32.mouse_event(0x0004, 0, 0, 0, 0)
    time.sleep(1.0)

def click_until_change(ly, tries=4):
    """点击直到画面变化；不变就逐步下移重试"""
    before = sig(grab_bytes())
    dy = 0
    for i in range(tries):
        click_logical(44, ly + dy)
        after = sig(grab_bytes())
        if after != before:
            print(f"  y={ly + dy} 生效")
            return
        print(f"  y={ly + dy} 无变化，重试")
        dy += 18
    print("  WARN no change")

home_y = 46 + 16 + 34
hist_y = home_y + 72
settings_y = LH - 48
about_y = settings_y - 72

save(grab_bytes(), "app-home.png")
# 预热点击标题栏空白，确保窗口获得焦点
click_logical(300, 23)
time.sleep(0.5)
click_until_change(hist_y)
save(grab_bytes(), "app-history.png")
click_until_change(settings_y)
save(grab_bytes(), "app-settings.png")
click_until_change(about_y)
save(grab_bytes(), "app-about.png")
click_logical(44, home_y)

user32.SetWindowPos(hwnd, -2, 0, 0, 0, 0, 0x0001 | 0x0002)
if prev_fore:
    cur_tid = kernel32.GetCurrentThreadId()
    fore_tid = user32.GetWindowThreadProcessId(user32.GetForegroundWindow(), None)
    user32.AttachThreadInput(cur_tid, fore_tid, True)
    user32.SetForegroundWindow(prev_fore)
    user32.AttachThreadInput(cur_tid, fore_tid, False)
print("done")
