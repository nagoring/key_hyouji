import { listen } from "@tauri-apps/api/event";

type RingPayload = { button: "left" | "right" | "middle"; color: string };

const ring = document.getElementById("ring")!;

await listen<RingPayload>("ring", (e) => {
  ring.style.setProperty("--color", e.payload.color);
  ring.className = `ring ${e.payload.button}`;
  void ring.offsetWidth; // アニメーションを最初から再生する
  ring.classList.add("play");
});
