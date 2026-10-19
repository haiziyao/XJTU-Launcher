// Review artwork only. Not application UI code.
import AppKit
let root = URL(fileURLWithPath: CommandLine.arguments[1])
let W: CGFloat = 1120, H: CGFloat = 760
func color(_ hex: UInt32) -> NSColor { NSColor(red: CGFloat((hex >> 16) & 255)/255, green: CGFloat((hex >> 8) & 255)/255, blue: CGFloat(hex & 255)/255, alpha: 1) }
let ink: UInt32 = 0x1f2329, muted: UInt32 = 0x7d8590, blue: UInt32 = 0x1f6fe5, line: UInt32 = 0xe4e6ea
func box(_ x: CGFloat,_ y: CGFloat,_ w: CGFloat,_ h: CGFloat,_ fill: UInt32 = 0xffffff,_ r: CGFloat = 7,_ stroke: Bool = false) {
 let path = NSBezierPath(roundedRect: NSRect(x:x,y:H-y-h,width:w,height:h),xRadius:r,yRadius:r); color(fill).setFill();path.fill()
 if stroke { color(line).setStroke();path.lineWidth=0.8;path.stroke() }
}
func text(_ s: String,_ x: CGFloat,_ y: CGFloat,_ size: CGFloat = 13,_ c: UInt32 = 0x1f2329,_ bold: Bool = false) {
 let font = NSFont.systemFont(ofSize:size,weight:bold ? .semibold:.regular)
 (s as NSString).draw(at:NSPoint(x:x,y:H-y-size-4),withAttributes:[.font:font,.foregroundColor:color(c)])
}
func icon(_ s:String,_ x:CGFloat,_ y:CGFloat,_ size:CGFloat = 17,_ c:UInt32 = 0x5c636d) {
 if let image=NSImage(systemSymbolName:s,accessibilityDescription:nil) {
 let tinted=NSImage(size:NSSize(width:size,height:size));tinted.lockFocus();image.draw(in:NSRect(x:0,y:0,width:size,height:size));color(c).setFill();NSRect(x:0,y:0,width:size,height:size).fill(using:.sourceAtop);tinted.unlockFocus();tinted.draw(in:NSRect(x:x,y:H-y-size,width:size,height:size))
 }
}
func button(_ s:String,_ x:CGFloat,_ y:CGFloat,_ width:CGFloat = 85,_ primary:Bool = false){box(x,y,width,30,primary ? blue:0xffffff,6,!primary);text(s,x+12,y+5,12,primary ? 0xffffff:ink)}
func divider(_ y:CGFloat,_ x:CGFloat = 228,_ w:CGFloat = 860){box(x,y,w,1,line,0)}
func base(_ active:String,_ title:String) {
 box(0,0,W,H,0xffffff,12);box(0,28,196,H-28,0xf1f2f4,0);box(195,28,1,H-28,line,0)
 for (i,c) in [UInt32(0xff6059),0xffbd2e,0x28c840].enumerated(){box(CGFloat(13+i*19),9,11,11,c,6)}
 let entries:[(String,String,CGFloat)] = [("主页","house",88),("Agent","cpu",126),("社区","bubble.left.and.bubble.right",164),("课程表","calendar",244),("校园导航","safari",282),("成绩查询","chart.bar.doc.horizontal",320),("一键评教","checklist",358),("插件市场","square.grid.2x2",626),("应用设置","gearshape",664)]
 text("插件",16,211,11,muted)
 for (name,symbol,y) in entries { if active == name {box(8,y-5,180,32,0xe1e8f3)};icon(symbol,20,y+2,17,active==name ? blue:0x5c636d);text(name,48,y,13,active==name ? blue:ink) }
 box(185,380,21,35,0xf7f8fa,5,true);icon("chevron.left",190,392,10)
 box(10,710,176,40,0xe9ebef);icon("person.crop.circle",21,718,24);text("校园账号",55,714,12,ink,true);text("当前账号",55,731,10,muted)
 text(title,228,59,22,ink,true)
}
func render(_ name:String,_ body:()->Void){let image=NSImage(size:NSSize(width:W,height:H));image.lockFocus();body();image.unlockFocus();let rep=NSBitmapImageRep(data:image.tiffRepresentation!)!;try! rep.representation(using:.png,properties:[:])!.write(to:root.appendingPathComponent(name+".png"))}
render("01-plugin-market") {
 base("插件市场","插件")
 box(228,107,222,31,0xf1f2f4);box(230,109,107,27,0xffffff,5);text("校园功能",254,113,12,ink,true);text("已安装",365,113,12,muted)
 box(840,59,248,30,0xf7f8fa,6,true);icon("magnifyingglass",850,67,14);text("搜索校园功能",874,65,12,muted)
 text("学习与教务",228,168,12,muted)
 let items=[("calendar","课程表","课程安排与考试时间","已启用"),("checklist","一键评教","选择问卷、批量填写与提交","添加"),("chart.bar.doc.horizontal","成绩查询","本研成绩与自选课程统计","添加"),("person.badge.clock","考勤查询","课程考勤与签到记录","添加"),("building.2","空闲教室","按校区、日期和节次查询","添加")]
 for (i,row) in items.enumerated(){let y=CGFloat(198+i*63);icon(row.0,245,y+10,23,blue);text(row.1,285,y,14,ink,true);text(row.2,285,y+23,12,muted);button(row.3,1006,y+8,70);divider(y+54)}
 text("校园生活",228,538,12,muted)
 for (i,row) in [("creditcard","校园卡","余额与消费记录"),("books.vertical","图书馆","座位查询与预约")].enumerated(){let y=CGFloat(567+i*65);icon(row.0,245,y+10,23,blue);text(row.1,285,y,14,ink,true);text(row.2,285,y+23,12,muted);button("添加",1006,y+8,70);divider(y+54)}
}
render("02-evaluation") {
 base("一键评教","一键评教");button("刷新",1014,58,74)
 text("2026–2027 学年 · 第一学期",228,105,12,muted)
 box(228,146,860,40,0xf7f8fa);text("待评教",245,156,12,ink,true);text("已完成",333,156,12,muted);text("全部类型",993,156,12,muted)
 let rows=[("高等数学","张老师 · 期末评教"),("大学物理","李老师 · 期末评教"),("程序设计","王老师 · 过程性评教")]
 for (i,row) in rows.enumerated(){let y=CGFloat(210+i*72);icon("checkmark.square.fill",245,y+8,18,blue);text(row.0,278,y,14,ink,true);text(row.1,278,y+24,12,muted);text("查看问卷",1005,y+12,12,blue);divider(y+58)}
 text("批量填写",228,452,14,ink,true);text("评分模板",228,496,13);button("良好",928,490,160)
 divider(537);text("文字意见",228,558,13);text("点击填写你的评价",934,558,12,muted);divider(600)
 text("已选 3 门课程",228,663,12,muted);button("预览答案",861,654,100);button("提交已选",975,654,113,true)
}
render("03-agent") {
 base("Agent","Agent");button("历史记录",852,58,99);button("新对话",967,58,121)
 text("DeepSeek",228,107,12,muted);icon("chevron.down",294,111,9);text("工具与记忆",1000,107,12,blue);divider(140)
 box(650,171,395,51,0xf1f2f4,10);text("帮我看看明天的课，再记住我喜欢简洁回答。",668,187,13)
 icon("cpu",265,259,20,blue);text("查询课程表",301,256,13,ink,true);text("完成",986,256,12,muted)
 box(301,291,726,89,0xf7f8fa,8);text("明天有 2 门课",319,306,14,ink,true);text("08:00  高等数学    ·    教学主楼 A203",319,334,13);text("10:10  大学物理    ·    教学主楼 B105",319,356,13)
 icon("checkmark.circle",301,411,15,0x2c8653);text("已记住：回答尽量简洁。",325,407,13)
 icon("doc.on.doc",301,455,14);icon("arrow.clockwise",330,455,14)
 box(263,575,790,124,0xffffff,11,true);text("发送消息，或输入 / 选择 Skill",281,594,14,muted);icon("paperclip",281,659,17);text("工作目录",312,658,12,muted);button("发送",965,653,70,true)
}
render("04-agent-settings") {
 base("Agent","Agent 设置");button("返回对话",981,58,107)
 box(228,112,420,32,0xf1f2f4);box(230,114,102,28,0xffffff,5);text("MCP",264,119,12,ink,true);text("Skills",368,119,12,muted);text("记忆",476,119,12,muted);text("工具",578,119,12,muted)
 text("已连接的服务",228,182,13,ink,true);button("添加服务",978,172,110)
 icon("server.rack",245,238,24,blue);text("校园知识库",286,231,14,ink,true);text("Streamable HTTP",286,255,12,muted);text("已连接 · 8 个工具",871,240,12,muted);icon("chevron.right",1061,242,12);divider(294)
 icon("terminal",245,326,24,blue);text("本地文件工具",286,319,14,ink,true);text("stdio",286,343,12,muted);text("已连接 · 5 个工具",871,328,12,muted);icon("chevron.right",1061,330,12);divider(382)
 text("添加 MCP 服务",228,429,15,ink,true)
 text("名称",228,474,13);text("点击填写",981,474,12,muted);divider(511)
 text("连接方式",228,541,13);button("Streamable HTTP",885,533,203);divider(579)
 text("地址",228,610,13);text("点击填写服务地址",934,610,12,muted);divider(650)
 button("取消",908,683,78);button("连接",1000,683,88,true)
}
