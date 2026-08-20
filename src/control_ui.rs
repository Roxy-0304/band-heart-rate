pub const CONTROL_HTML: &str = r##"<!DOCTYPE html>
<html lang="zh">
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Band Heart Rate 控制面板</title>
    <link rel="preconnect" href="https://fonts.googleapis.com" />
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet" />
    <style>
        :root {
            --bg-0: #09090b; --bg-1: #18181b; --bg-2: #27272a; --bg-3: #3f3f46;
            --fg-0: #fafafa; --fg-1: #a1a1aa; --fg-2: #71717a;
            --accent: #3b82f6; --accent-hover: #2563eb; --accent-dim: rgba(59,130,246,0.12);
            --green: #22c55e; --green-dim: rgba(34,197,94,0.12);
            --red: #ef4444; --red-dim: rgba(239,68,68,0.12);
            --yellow: #eab308; --yellow-dim: rgba(234,179,8,0.12);
            --radius: 8px; --radius-sm: 5px;
            --font: "Inter", -apple-system, "Microsoft YaHei", sans-serif;
            --transition: 150ms ease-out;
        }
        *, *::before, *::after { margin: 0; padding: 0; box-sizing: border-box; }
        html { font-size: 15px; -webkit-font-smoothing: antialiased; }
        body { font-family: var(--font); background: var(--bg-0); color: var(--fg-0); line-height: 1.5; padding: 24px; min-height: 100vh; }
        ::selection { background: var(--accent); color: #fff; }
        ::-webkit-scrollbar { width: 6px; } ::-webkit-scrollbar-track { background: transparent; } ::-webkit-scrollbar-thumb { background: var(--bg-3); border-radius: 3px; }

        .page { max-width: 640px; margin: 0 auto; }
        .page-title { font-size: 1.25rem; font-weight: 600; letter-spacing: -0.02em; margin-bottom: 24px; }

        /* Section */
        .section { margin-bottom: 20px; }
        .section-label { font-size: 0.7rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.08em; color: var(--fg-2); margin-bottom: 8px; padding-left: 2px; }

        /* Card */
        .card { background: var(--bg-1); border: 1px solid var(--bg-2); border-radius: var(--radius); padding: 14px 16px; }

        /* Status rows */
        .stat-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 1px; background: var(--bg-2); border-radius: var(--radius); overflow: hidden; }
        .stat-cell { background: var(--bg-1); padding: 12px 14px; display: flex; flex-direction: column; gap: 2px; }
        .stat-label { font-size: 0.7rem; color: var(--fg-2); font-weight: 500; }
        .stat-value { font-size: 0.93rem; font-weight: 600; font-variant-numeric: tabular-nums; }
        .stat-value.ok { color: var(--green); } .stat-value.err { color: var(--red); } .stat-value.warn { color: var(--yellow); } .stat-value.dim { color: var(--fg-2); }

        /* Device list */
        .device-list { list-style: none; }
        .device-item { display: flex; justify-content: space-between; align-items: center; padding: 10px 12px; border-radius: var(--radius-sm); cursor: pointer; transition: background var(--transition); border: 1px solid transparent; }
        .device-item + .device-item { margin-top: 2px; }
        .device-item:hover { background: var(--bg-2); }
        .device-item:active { background: var(--bg-3); }
        .device-item.selected { background: var(--accent-dim); border-color: var(--accent); }
        .device-name { font-size: 0.87rem; font-weight: 500; }
        .device-id { font-size: 0.67rem; color: var(--fg-2); font-family: "SF Mono", "Cascadia Code", monospace; margin-top: 1px; }
        .device-empty { color: var(--fg-2); font-size: 0.8rem; text-align: center; padding: 24px 0; }

        /* Buttons */
        .btn-row { display: flex; gap: 8px; margin-top: 10px; }
        .btn { display: inline-flex; align-items: center; justify-content: center; gap: 6px; padding: 7px 14px; border: none; border-radius: var(--radius-sm); font-family: var(--font); font-size: 0.8rem; font-weight: 500; cursor: pointer; transition: background var(--transition), opacity var(--transition); }
        .btn:active { transform: scale(0.97); }
        .btn-primary { background: var(--accent); color: #fff; } .btn-primary:hover { background: var(--accent-hover); }
        .btn-danger { background: var(--red-dim); color: var(--red); } .btn-danger:hover { background: var(--red); color: #fff; }
        .btn:disabled { opacity: 0.4; cursor: not-allowed; }

        /* Form */
        .field { margin-bottom: 10px; }
        .field-label { display: block; font-size: 0.73rem; font-weight: 500; color: var(--fg-1); margin-bottom: 4px; }
        .input { width: 100%; padding: 8px 10px; background: var(--bg-0); border: 1px solid var(--bg-3); border-radius: var(--radius-sm); color: var(--fg-0); font-family: var(--font); font-size: 0.87rem; transition: border-color var(--transition); }
        .input:focus { outline: none; border-color: var(--accent); box-shadow: 0 0 0 2px var(--accent-dim); }
        .input::placeholder { color: var(--fg-2); }

        /* Toast */
        .toast { position: fixed; bottom: 20px; left: 50%; transform: translateX(-50%) translateY(8px); background: var(--bg-2); color: var(--fg-0); padding: 8px 16px; border-radius: var(--radius); font-size: 0.8rem; font-weight: 500; opacity: 0; transition: opacity 200ms ease, transform 200ms ease; pointer-events: none; border: 1px solid var(--bg-3); }
        .toast.show { opacity: 1; transform: translateX(-50%) translateY(0); }

        /* Focus ring */
        :focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
    </style>
</head>
<body>
    <div class="page">
        <div class="page-title">Band Heart Rate</div>

        <div class="section">
            <div class="section-label">状态</div>
            <div class="stat-grid">
                <div class="stat-cell"><span class="stat-label">连接</span><span class="stat-value" id="s-conn">--</span></div>
                <div class="stat-cell"><span class="stat-label">心率</span><span class="stat-value" id="s-hr">--</span></div>
                <div class="stat-cell"><span class="stat-label">设备</span><span class="stat-value dim" id="s-dev">--</span></div>
                <div class="stat-cell"><span class="stat-label">扫描</span><span class="stat-value" id="s-scan">--</span></div>
            </div>
            <div id="s-err-row" style="display:none;margin-top:8px;"><div class="card" style="border-color:var(--red);background:var(--red-dim);padding:10px 12px;"><span class="stat-label" style="color:var(--red);">Error</span><div class="stat-value err" id="s-err" style="font-size:0.8rem;margin-top:2px;font-weight:400;"></div></div></div>
        </div>

        <div class="section">
            <div class="section-label">设备列表</div>
            <div class="card" style="padding:8px;">
                <ul class="device-list" id="dlist"><li class="device-empty">正在扫描…</li></ul>
            </div>
            <div class="btn-row">
                <button class="btn btn-danger" id="btn-disconnect" disabled>断开连接</button>
            </div>
        </div>

        <div class="section">
            <div class="section-label">设置</div>
            <div class="card">
                <div class="field"><label class="field-label">最大心率</label><input class="input" id="c-mhr" type="number" min="60" max="250" /></div>
                <div class="field"><label class="field-label">允许设备（逗号分隔）</label><input class="input" id="c-dev" type="text" placeholder="band,amazfit,watch,mi" /></div>
                <div class="field"><label class="field-label">服务端口</label><input class="input" id="c-port" type="number" min="1024" max="65535" /></div>
                <div class="btn-row"><button class="btn btn-primary" id="btn-save">保存</button></div>
            </div>
        </div>
    </div>

    <div class="toast" id="toast"></div>

    <script>
    (function(){
        var $=function(s){return document.getElementById(s)};
        var toastTimer=null;
        function toast(m){
            var t=$("toast");t.textContent=m;t.classList.add("show");
            clearTimeout(toastTimer);toastTimer=setTimeout(function(){t.classList.remove("show")},1800);
        }
        function esc(s){var d=document.createElement("span");d.textContent=s;return d.innerHTML}

        var currentAddr=null;

        async function refresh(){
            try{
                var r=await fetch("/heart-rate");var hr=await r.json();
                var r2=await fetch("/devices");var devs=await r2.json();
                var r3=await fetch("/settings");var cfg=await r3.json();

                var c=$("s-conn");
                c.textContent=hr.connected?"已连接":"未连接";
                c.className="stat-value "+(hr.connected?"ok":"err");

                $("s-hr").textContent=hr.heart_rate>0?hr.heart_rate+" BPM":"--";
                $("s-hr").className="stat-value "+(hr.heart_rate>0?"":"dim");

                var dn=hr.device_name||"--";
                $("s-dev").textContent=dn;
                $("s-dev").className="stat-value "+(hr.device_name?"":"dim");

                var sc=$("s-scan");
                sc.textContent=hr.scanning?"是":"否";
                sc.className="stat-value "+(hr.scanning?"warn":"dim");

                var er=$("s-err-row");
                if(hr.error){er.style.display="block";$("s-err").textContent=hr.error;}
                else{er.style.display="none";}

                currentAddr=hr.device_address||null;

                var l=$("dlist");
                if(devs.length===0){
                    l.innerHTML="<li class=\"device-empty\">"+(hr.scanning?"正在扫描…":"未找到设备")+"</li>";
                }else{
                    var h="";
                    for(var i=0;i<devs.length;i++){
                        var d=devs[i];
                        var sel=currentAddr===d.id?" selected":"";
                        h+="<li class=\"device-item"+sel+"\" data-id=\""+esc(d.id)+"\">";
                        h+="<div><div class=\"device-name\">"+esc(d.name||"未知设备")+"</div>";
                        h+="<div class=\"device-id\">"+esc(d.id)+"</div></div></li>";
                    }
                    l.innerHTML=h;
                }

                $("btn-disconnect").disabled=!hr.connected;

                $("c-mhr").value=cfg.max_heart_rate;
                $("c-dev").value=cfg.allowed_devices;
                $("c-port").value=cfg.server_port;
            }catch(e){console.error(e)}
        }

        $("dlist").addEventListener("click",function(e){
            var item=e.target.closest(".device-item");
            if(!item)return;
            var id=item.getAttribute("data-id");
            if(!id)return;
            fetch("/devices/select",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({device_id:id})})
                .then(function(){toast("正在连接…")})
                .catch(function(){toast("连接失败")});
        });

        $("btn-disconnect").addEventListener("click",function(){
            fetch("/devices/disconnect",{method:"POST"})
                .then(function(){toast("已断开")})
                .catch(function(){toast("操作失败")});
        });

        $("btn-save").addEventListener("click",function(){
            var v=parseInt($("c-port").value);
            if(isNaN(v)||v<1024||v>65535){toast("端口号无效");return;}
            var cfg={
                max_heart_rate:parseInt($("c-mhr").value)||190,
                allowed_devices:$("c-dev").value,
                server_port:v,
                auto_start:false,
                minimize_to_tray:true
            };
            fetch("/settings",{method:"PUT",headers:{"Content-Type":"application/json"},body:JSON.stringify(cfg)})
                .then(function(){toast("已保存")})
                .catch(function(){toast("保存失败")});
        });

        refresh();setInterval(refresh,2000);
    })();
    </script>
</body>
</html>"##;
