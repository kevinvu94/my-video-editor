# CLAUDE.md

Quy tắc làm việc cho repo này (fork của https://github.com/jub0t/Concat).

## Bảo mật: repo đang để Public

- Tuyệt đối không hardcode API key, token, mật khẩu hay thông tin nội bộ vào mã nguồn, file cấu hình, tài liệu hoặc commit message.
- Không commit font hoặc asset có bản quyền. Chỉ dùng tài nguyên có giấy phép cho phép phân phối lại.
- Bí mật và giá trị riêng của máy cần đưa qua biến môi trường hoặc file không được theo dõi bởi git (kiểm tra `.gitignore`).
- Trước khi commit, rà lại diff để chắc không lộ gì ở các mục trên.

## Nhánh làm việc

- Luôn làm việc trên nhánh `custom-main`.
- Không sửa trực tiếp nhánh `main`. `main` được giữ sạch, trùng với bản gốc, để bấm Sync fork lấy bản vá lỗi từ upstream.
- Muốn lấy bản vá: bấm Sync fork trên GitHub, rồi cập nhật `main` và merge hoặc rebase vào `custom-main`.
- Remote `upstream` trỏ về bản gốc.
