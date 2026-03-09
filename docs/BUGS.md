# 🐛 Squad Station — Bugs Found During Testing

> Phát hiện: 2026-03-09 | Project: squad-station-web (landing page)

## 📌 Status Legend
- `[ ]` **Open**: Bug is confirmed nhưng chưa fix.
- `[~]` **In Progress**: Đang tiến hành fix.
- `[x]` **Fixed**: Đã fix và test thành công.

---

## [x] Bug #1: `squad-station view` — Nested tmux attach fails silently

**Severity:** Medium  
**Command:** `squad-station view`  
**File:** [view.rs](file:///Users/tranthien/Documents/2.DEV/2.PRIVATE/squad-station/src/commands/view.rs) + [tmux.rs](file:///Users/tranthien/Documents/2.DEV/2.PRIVATE/squad-station/src/tmux.rs#L197-L225)

**Mô tả:**  
- `view` tạo window tên `squad-view` bằng `tmux new-window` với command `tmux attach-session -t <agent>`
- Nhưng nested tmux attach bị chặn vì biến `$TMUX` đã set — session chết ngay lập tức
- User chạy `tmux attach -t squad-view` → "can't find session" vì đây là **window**, không phải session

**Workaround hiện tại:**  
Tạo session `squad-monitor` riêng, dùng `TMUX=` để unset env trước khi attach:
```bash
tmux new-session -d -s squad-monitor
tmux send-keys -t squad-monitor "TMUX= tmux attach -t <agent1>" Enter
tmux split-window -h -t squad-monitor
tmux send-keys -t squad-monitor "TMUX= tmux attach -t <agent2>" Enter
```

**Fix đề xuất:**  
- Tạo **session** thay vì window.
- **Naming Convention:** Cần sử dụng tiền tố theo tên project (ví dụ: `squad-monitor-<project-name>`) thay vì hardcode `squad-monitor`, để tránh xung đột conflict khi người dùng chạy `squad-station view` cho nhiều dự án khác nhau trên cùng một máy (mỗi project một squad station riêng).
- Dùng `TMUX=` unset trước khi nested attach
- Hoặc thay `attach-session` bằng `capture-pane -p -t <agent>` + watch loop

---

## [x] Bug #2: `squad-station signal` — Xử lý `$TMUX_PANE` phức tạp và không cần thiết

**Severity:** Low (Code Complexity / Logic Flaw)  
**Command:** `squad-station signal` / `squad-station init`  
**File:** [signal.rs](file:///Users/tranthien/Documents/2.DEV/2.PRIVATE/squad-station/src/commands/signal.rs) và [init.rs](file:///Users/tranthien/Documents/2.DEV/2.PRIVATE/squad-station/src/commands/init.rs)

**Mô tả:**  
- Hiện tại logic lấy tên agent đang phụ thuộc quá nhiều vào biến môi trường `$TMUX_PANE` (ví dụ: parse `%3` để mò ra tên session).
- Điều này vừa khiến code trong `signal.rs` phức tạp (Guard 1 có nhiều nhánh match) vừa sinh ra lỗi silent exit khi dev chạy test thủ công ở môi trường không có bash `$TMUX_PANE`.
- Việc inject command vào settings thông qua `init.rs` đang dùng hardcode string `squad-station signal $TMUX_PANE`.

**Fix đề xuất:**  
- **Bên phía Hook (`init.rs`)**: Sửa lại command sinh hook tự động thành việc lấy thẳng tên session tại thời điểm gọi. Ví dụ: đổi command thành `"squad-station signal $(tmux display-message -p '#S')"` (hoặc một cú pháp tương đương có thể resolve ra tên session động dựa theo shell context).
- **Bên phía Agent (`signal.rs`)**: Xoá sổ hoàn toàn logic phụ thuộc vào biến `$TMUX_PANE`. Thay vào đó, hàm `signal` chỉ việc nhận vào tham số arg thứ nhất là tên của agent/session một cách tường minh và xử lý thẳng. Nếu không cung cấp tên → văng lỗi / silent exit luôn.

---

## [x] Bug #3: `squad-station init` — tmux `has-session` stderr leaks to user output

**Severity:** Cosmetic (User Experience)  
**Command:** `squad-station init`  
**File:** [tmux.rs](file:///Users/tranthien/Documents/2.DEV/2.PRIVATE/squad-station/src/tmux.rs#L166-L172)

**Mô tả:**  
- Trong lệnh `init`, framework sẽ kiểm tra xem agent session đã tồn tại hay chưa thông qua hàm `session_exists()` (sử dụng lệnh `tmux has-session -t <name>`).
- Nếu session đó chưa được tạo (đây là điều hiển nhiên trong lần chạy `init` đầu tiên), tiến trình `tmux` sẽ báo lỗi `"can't find session: <name>"` vào luồng **`stderr`**.
- Do hàm này gọi `.status()` mà không chặn `stderr`, dòng thông báo này bị lọt (leak) trực tiếp ra màn hình terminal của người dùng.
- Kết quả: Khi user chạy `squad-station init` lần đầu, console hiện ra một loạt thông báo lỗi màu đỏ/gây hoang mang, làm user tưởng rằng lệnh init thất bại (dù thực tế script vẫn tiếp tục khởi tạo session thành công).

**Fix đề xuất:**  
- **Tắt stderr leak**: Sửa hàm `session_exists()` trong `tmux.rs` để chặn luồng stderr. Thay vì dùng `Command::new(...).status()`, hãy dùng `Command::new(...).output()`.
- Lệnh `output()` sẽ bắt cả `stdout` và `stderr` vào biến bộ nhớ và không in gì ra màn hình console, khắc phục hoàn toàn hiện tượng leak tin nhắn báo lỗi này.

---

## [x] Bug #4: `squad.yml` Config Format Contradiction (v1.1+ Schema)

**Severity:** High (Parsing Error)  
**Command:** `squad-station init`  
**File:** [squad.yml](file:///Users/tranthien/Documents/2.DEV/2.PRIVATE/squad-station-landing-page/squad.yml) và [config.rs](file:///Users/tranthien/Documents/2.DEV/2.PRIVATE/squad-station/src/config.rs)

**Mô tả:**  
- Trong file `squad.yml` của project sử dụng Squad Station đang dùng keyword `provider` (`provider: antigravity`, vv), đây là định dạng chuẩn xác.
- Tuy nhiên, trong core `squad-station` v1.1+ (file `src/config.rs`), struct `AgentConfig` đã bị đổi tên trường `provider` thành `tool` một cách không hợp lý.
- Do đó khi parse file yaml sẽ báo lỗi không deserialize được.

**Fix đề xuất:**  
- Từ "tool" không phản ánh đúng ngầm định của schema, "provider" mới là từ chính xác.
- (Cho Core): Cần sửa lại struct `AgentConfig` trong `src/config.rs`, đổi tên field `tool` quay trở lại thành `provider`.
- Thay đổi liên đới: Cập nhật hàm `is_db_only()` và các đoạn code parse config khác để tương thích với field `provider` mới.

---

## [x] Bug #5: Missing Configuration Validation (Strict Mode)

**Severity:** High (Silent Failure / False Positive)  
**Command:** `squad-station init`  
**File:** [config.rs](file:///Users/tranthien/Documents/2.DEV/2.PRIVATE/squad-station/src/config.rs)

**Mô tả:**  
- Core `squad-station` chưa có logic validate nội dung (giá trị thực) của trường `provider` (và `model`) khi deserialize file `squad.yml`.
- Nếu người dùng typo hoặc nhập sai tên provider (ví dụ: `gemini`, `claude-code-2` thay vì chuẩn là `gemini-cli`, `claude-code`), struct `AgentConfig` vẫn parse thành chuỗi `String` hợp lệ và vượt qua bài test định dạng.
- Lệnh `squad-station init` dùng luôn string sai này để run background tmux. Tmux sẽ vẫn tạo session nhưng app bên trong báo command not found rồi tắt lịm luôn. Tuy nhiên script vẫn báo ngoài CLI là "Initialized squad with x agents" — tức là một false positive báo thành công giả tạo!

**Fix đề xuất:**  
- **Về phía Provider**: Cần validate chặt chẽ field `provider`. Chỉ chấp nhận đúng whitelist: `antigravity`, `claude-code`, `gemini-cli`. Bất cứ text nào khác rơi vào đây, rust phải văng lỗi validate, **chặn tiến trình init**, và **gợi ý (suggest) rõ ràng ra console cho người dùng** danh sách các provider hợp lệ để họ sửa lại.
- **Về phía Model**: Cần thiết lập danh sách model hợp lệ và validate tương ứng với từng provider. Nếu người dùng nhập sai model, hãy in ra thông báo lỗi kèm danh sách gợi ý model hợp lệ tương ứng với provider đó:
  - `claude-code` = `opus`, `sonnet`, `haiku`
  - `gemini-cli` = `gemini-3.1-pro-preview`, `gemini-3-flash-preview`
