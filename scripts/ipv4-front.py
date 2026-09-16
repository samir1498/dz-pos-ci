# IPv4 front door for Metro. Metro listens on a dual-stack `::` socket, which
# WSL's relay mirrors onto Windows as [::1] only; `adb reverse` targets
# 127.0.0.1, so the emulator reached nothing. This listens on IPv4 and hands
# each connection to Metro.
import asyncio, sys
LISTEN, TARGET = int(sys.argv[1]), int(sys.argv[2])
async def pipe(r, w):
    try:
        while data := await r.read(65536):
            w.write(data); await w.drain()
    except Exception:
        pass
    finally:
        w.close()
async def handle(cr, cw):
    try:
        tr, tw = await asyncio.open_connection("127.0.0.1", TARGET)
    except Exception:
        cw.close(); return
    await asyncio.gather(pipe(cr, tw), pipe(tr, cw))
async def main():
    srv = await asyncio.start_server(handle, "0.0.0.0", LISTEN)
    async with srv: await srv.serve_forever()
asyncio.run(main())
