---@param end_time integer time in milliseconds, from utc
local function spinSleepUntil(end_time)
    while os.epoch("utc") < end_time - 500 do
        coroutine.yield()
    end
    while os.epoch("utc") < end_time do end
end

---@param time integer time in milliseconds
local function spinSleep(time)
    local start = os.epoch("utc")
    local end_time = start + time
    spinSleepUntil(end_time)
end

---@class MonitorArray: ccTweaked.term.Redirect
---@field monitor_rows ccTweaked.peripheral.Monitor[][] row-major y-down monitor array
---@field cursorPos number[]
MonitorArray = {}

---@param monitors ccTweaked.peripheral.Monitor[]
function MonitorArray:setup(monitors)
    print("click on monitors, starting from the top left, going down, clicking twice on the top right corner")

    ---@type {__index: ccTweaked.peripheral.Monitor}
    local monitor_map = {}
    for _, monitor in ipairs(monitors) do
        for i = 1, 16, 1 do
            local index = 2 ^ (i - 1)
            local r, g, b = term.nativePaletteColor(index)
            monitor.setPaletteColor(index, r, g, b)
        end
        monitor.setTextColor(colors.black)
        monitor.setBackgroundColor(colors.black)
        monitor.clear()
        monitor_map[peripheral.getName(monitor)] = monitor
    end

    ---@type ccTweaked.peripheral.Monitor[]
    local sorted_monitors = {}
    ---@type ccTweaked.peripheral.Monitor?
    local last_monitor

    local width, height = 0, 0

    local count = 0

    while true do
        ---@type ccTweaked.os.event, string, integer, integer
        local _event, side = os.pullEvent("monitor_touch")
        count = count + 1

        local touched = monitor_map[side]
        if touched == nil then goto continue end

        if last_monitor == touched and width == 0 then
            width = count - 1
            touched.setBackgroundColor(colors.blue)
            touched.clear()
            goto continue
        end

        touched.setBackgroundColor(colors.green)
        touched.clear()
        sorted_monitors[#sorted_monitors+1] = touched
        last_monitor = touched
        if #sorted_monitors == #monitors then
            if width == 0 then
                width = #sorted_monitors
            end
            break
        end
        ::continue::
    end
    height = math.ceil(#monitors / width)

    return MonitorArray:new(sorted_monitors, width, height)
end

---@param monitors ccTweaked.peripheral.Monitor[]
---@param width integer
---@param height integer
---@return MonitorArray
function MonitorArray:new(monitors, width, height)
    assert(
        width * height == #monitors,
        string.format("monitor array (length %d) doesnt match width and height (%d, %d)", #monitors, width, height)
    )
    local rows = {}
    local idx = 1
    for y=1, height do
        local row = {}
        for x=1, width do
            row[#row+1] = monitors[idx]
            idx = idx + 1
        end
        rows[#rows+1] = row
    end
    ---@type MonitorArray
    local t = {
        cursorPos = {1, 1},
        monitor_rows = rows
    }
    setmetatable(t, self)
    self.__index = self

    -- print(textutils.serialize(t.monitor_rows))

    t:forEach(function (_, _, monitor)
        for i = 1, 16, 1 do
            local index = 2 ^ (i - 1)
            local r, g, b = term.nativePaletteColor(index)
            monitor.setPaletteColor(index, r, g, b)
        end
        monitor.setBackgroundColor(colors.black)
        monitor.setTextColor(colors.white)
        monitor.setCursorPos(1, 1)
        monitor.setCursorBlink(false)
        monitor.clear()
        monitor.setTextScale(0.5)
    end)

    t.write = function(text)
        t:forEach(
            function (_, _, monitor)
                monitor.write(text)
            end
        )
        coroutine.yield()
    end

    t.blit = function (text, textColor, bgColor)
        t:forEach(function (_, _, monitor)
            monitor.blit(text, textColor, bgColor)
        end)
    end

    t.isColor = function()
        return true
    end
    t.isColour = t.isColor

    t.setTextColor = function(color)
        t:forEach(function (_, _, monitor)
            monitor.setTextColor(color)
        end)
    end
    t.setTextColour = t.setTextColor

    t.getTextColor = function()
        return t.monitor_rows[1][1].getTextColor()
    end
    t.getTextColour = t.getTextColor

    t.clear = function()
        t:forEach(function(_, _, monitor) monitor.clear() end)
    end

    t.setBackgroundColor = function(color)
        t:forEach(function(_, _, monitor) monitor.setBackgroundColor(color) end)
    end
    t.setBackgroundColour = t.setBackgroundColor

    t.getBackgroundColor = function()
        return t.monitor_rows[1][1].getBackgroundColor()
    end
    t.getBackgroundColour = t.getBackgroundColor

    t.getCursorPos = function()
        return t.cursorPos[1], t.cursorPos[2]
    end

    t.setCursorPos = function (x, y)
        t.cursorPos[1] = x
        t.cursorPos[2] = y
        t:forEach(function (xPos, yPos, monitor)
            local prevW = 0
            local prevH = 0
            for i=1, yPos-1 do
                prevH = prevH + select(2, t:getMonitor(xPos, i).getSize())
            end
            for i=1, xPos-1 do
                prevW = prevW + select(1, t:getMonitor(i, yPos).getSize())
            end
            monitor.setCursorPos(x - prevW, y - prevH)
        end)
    end

    t.scroll = function (y)
        t:forEach(function (_, _, monitor)
            monitor.scroll(y)
        end)
    end

    t.getCursorBlink = function()
        return t.monitor_rows[1][1].getCursorBlink()
    end

    t.setCursorBlink = function(blink)
        t:forEach(function (_, _, monitor)
            monitor.setCursorBlink(blink)
        end)
    end

    t.setPaletteColor = function(index, col_or_r, g, b)
        t:forEach(function(_, _, monitor)
            if g ~= nil then
                monitor.setPaletteColor(
                    index,
                    assert(tonumber(col_or_r)),
                    assert(tonumber(g)),
                    assert(tonumber(b))
                )
            else
                monitor.setPaletteColor(
                    index,
                    assert(tonumber(col_or_r))
                )
            end
        end)
    end
    t.setPaletteColour = t.setPaletteColor

    t.getPaletteColor = function(index)
        return t.monitor_rows[1][1].getPaletteColor(index)
    end
    t.getPaletteColour = t.getPaletteColor

    t.getSize = function()
        local biggest_width = 0
        local biggest_height = 0
        for _, row in ipairs(t.monitor_rows) do
            local width = 0
            for _, monitor in ipairs(row) do
                local w, h = monitor.getSize()
                width = width + w
            end
            if width > biggest_width then
                biggest_width = width
            end
        end

        ---@type ccTweaked.peripheral.Monitor[]
        local columns = {}
        local curColumn = 0
        while true do
            curColumn = curColumn + 1
            if curColumn > #t.monitor_rows[1] then
                break
            end
            ---@type ccTweaked.peripheral.Monitor[]
            local column = {}
            for _, row in ipairs(t.monitor_rows) do
                column[#column+1] = row[curColumn]
            end
            if #column == 0 then
                break
            else
                columns[#columns+1] = column
            end
        end

        for _, column in ipairs(columns) do
            local height = 0
            for _, monitor in ipairs(column) do
                local w, h = monitor.getSize()
                height = height + h
            end
            if height > biggest_height then
                biggest_height = height
            end
        end

        return biggest_width, biggest_height
    end

    MonitorArray.saveToSettings(t)
    return t
end

---@param func fun(x: integer, y: integer, monitor: ccTweaked.peripheral.Monitor)
function MonitorArray:forEach(func)
    for y, row in ipairs(self.monitor_rows) do
        for x, monitor in ipairs(row) do
            func(x, y, monitor)
        end
    end
end

---@param x integer
---@param y integer
---@return ccTweaked.peripheral.Monitor
function MonitorArray:getMonitor(x, y)
    return self.monitor_rows[y][x]
end

function MonitorArray:rows()
    return ipairs(self.monitor_rows), self, 0
end

MARRAY_LAYOUT_SETTING = "marray_layout"

---@return MonitorArray?
---@nodiscard
function MonitorArray.loadFromSettings()
    local function unset_setting()
        settings.unset(MARRAY_LAYOUT_SETTING)
        settings.save()
        printError("failed to load from settings")
    end

    local loaded = settings.get(MARRAY_LAYOUT_SETTING)
    if loaded == nil or type(loaded) ~= "table" then
        unset_setting()
        return
    end

    local monitors = {}
    for _, value in ipairs(loaded.monitors) do
        if type(value.side) == string then
            unset_setting()
            return
        end

        local wrapped = peripheral.wrap(value.side)
        if wrapped == nil or peripheral.getType(value.side) ~= "monitor" then
            printError(value.side)
            unset_setting()
            return
        end

        local prevScale = wrapped.getTextScale()
        wrapped.setTextScale(1.0)
        local width, height = wrapped.getSize()
        wrapped.setTextScale(prevScale)

        if width ~= value.width or height ~= value.height then
            printError(width, value.width, height, value.height)
            unset_setting()
            return
        end

        monitors[#monitors+1] = wrapped;
    end
    return MonitorArray:new(monitors, loaded.width, loaded.height)
end

function MonitorArray:saveToSettings()
    local toSave = {
        height=#self.monitor_rows,
        width=#self.monitor_rows[1],
        monitors={}
    }

    self:forEach(function (_, _, monitor)
        local prevScale = monitor.getTextScale()
        monitor.setTextScale(1.0)
        local width, height = monitor.getSize()
        monitor.setTextScale(prevScale)

        toSave.monitors[#toSave.monitors+1] = {
            side=peripheral.getName(monitor),
            width=width,
            height=height
        }
    end)

    settings.set(
        MARRAY_LAYOUT_SETTING,
        toSave
    )
    settings.save()
end

function MonitorArray:columns()
    local curColumn = 0
    local ma = self
    term.native().write(textutils.serialize(self.cursorPos))
    local columns = {}

    while true do
        curColumn = curColumn + 1
        term.native().write("\n")
        if curColumn > #ma.monitor_rows[1] then
            return nil
        end
        ---@type ccTweaked.peripheral.Monitor[]
        local column = {}
        for _, row in ipairs(ma.monitor_rows) do
            column[#column+1] = row[curColumn]
        end
        if #column == 0 then
            break
        else
            columns[#columns+1] = column
        end
    end
    return ipairs(columns), self, 0
end

---@return ccTweaked.peripheral.Monitor[]
local function findAllMonitors()
    local name = peripheral.getNames()
    local monitors = {};
    for _, name in ipairs(name) do
        local per = assert(peripheral.wrap(name))
        if peripheral.getType(per) == "monitor" then
            monitors[#monitors+1] = per
        end
    end
    return monitors
end


---@class Dequeue<T>: {buf: T[]}
---@field start integer
---@field len integer
---@field cap integer
Dequeue = {}

---@generic T
---@return Dequeue<T>
---@param cap integer
function Dequeue:new(cap)
    ---@class Dequeue<T>
    local t = {
        start = 1,
        len = 0,
        cap = cap,
        buf = {}
    }
    setmetatable(t, self)
    self.__index = self

    ---@generic T
    ---@param val T
    ---@return boolean
    t.push_back = function (val)
        if t.len == cap then
            return false
        end
        t.buf[((t.start - 1 + t.len) % t.cap) + 1] = val
        t.len = t.len + 1
        return true
    end

    ---@generic T
    ---@param val T
    ---@return boolean
    t.push_front = function (val)
        if t.len == cap then
            return false
        end
        local idx = nil
        if t.start == 1 then
            idx = t.start + t.len - 1
        else
            idx = t.start - 1
        end
        t.buf[idx] = val
        t.start = idx
        t.len = t.len + 1
        return true
    end

    ---@generic T
    ---@return T | nil
    t.pop_front = function ()
        if t.len == 0 then
            return nil
        end
        local ret = t.buf[t.start]
        t.start = (t.start % t.cap) + 1
        t.len = t.len - 1
        return ret
    end

    return t
end

local monitors = findAllMonitors()
---@type ccTweaked.peripheral.Speaker | nil
local speaker = peripheral.find("speaker")
local default_term = term.current()
local output = term.current()
if #monitors == 1 then
    output = findAllMonitors[1]
elseif #monitors > 1 then
    output = MonitorArray.loadFromSettings() or MonitorArray:setup(monitors)
end

local function pcPrint(...)
    local prev = term.current()
    term.redirect(default_term)
    print(...)
    term.redirect(prev)
end

local width, height = output.getSize()

local stream_url = arg[1]
while stream_url == nil or stream_url == "" do
    print("stream url: ")
    stream_url = read()
end
print("starting stream: " .. stream_url)
stream_url = textutils.urlEncode(stream_url)
local ws = http.websocketAsync("wss://stuff.juliapixel.com/stream?url=" .. stream_url .. "&width=" .. width .. "&height=" .. height)

print("listening")

term.redirect(output)

local function displayFrame(frame)
    for i, value in ipairs(frame.palette) do
        term.setPaletteColor(
            2 ^ (i - 1),
            colors.packRGB(
                value[1] / 255.0,
                value[2] / 255.0,
                value[3] / 255.0
            )
        )
    end
    for i, value in ipairs(frame.rows) do
        term.setCursorPos(1, i)
        term.blit(string.rep(" ", string.len(value)), value, value)
    end
end

local dfpwm = require("cc.audio.dfpwm")

local decoder = dfpwm.make_decoder()

local ws_closed = false

if arg.test then
    local test_q = Dequeue:new(10)
    test_q.push_back(123)
    print(textutils.serialize(test_q.buf))
    print(test_q.cap)
    print(test_q.len)
    print(test_q.start)
    assert(test_q.pop_front() == 123, "AAA")
    test_q.push_back(321)
    test_q.push_back(456)
    assert(test_q.pop_front() == 321, "BBB")
end

local function wait_for_close()
    local _event, _url, message = os.pullEvent("websocket_close")
    print(message)
    ws_closed = true
end

---@type Dequeue<string>
local samples = Dequeue:new(25)
local waiting_for_samples = true

local function wait_for_audio()
    if waiting_for_samples then
        os.pullEvent("new_samples")
    end

    local popped = samples.pop_front()
    if popped == nil then
        waiting_for_samples = true
        pcPrint("waiting for samples")
        return
    end
    local decoded = decoder(popped)
    if not speaker.playAudio(decoded, 3) then
        os.pullEvent("speaker_audio_empty")
    else
        local popped = samples.pop_front()
        if popped == nil then
            waiting_for_samples = true
            pcPrint("waiting for samples")
            return
        end
        local decoded = decoder(popped)
        if not speaker.playAudio(decoded, 3) then
            samples.push_front(popped)
        end
    end
    waiting_for_samples = true
end

local sample_acc = ""

local function read_ws()
    local _e, _url, message, is_binary = os.pullEvent("websocket_message")
    if message == nil then
        ws_closed = true
    end
    if is_binary then
        return
    end
    local ending = os.epoch("utc") + 33;
    ---@type {palette: integer[][], rows: string[]} | {samples: integer[]}
    local message, errorMessage = textutils.unserialiseJSON(message);
    assert(message, errorMessage)
    if message.palette ~= nil then
        displayFrame(message)
        spinSleepUntil(ending)
    elseif message.samples ~= nil and speaker ~= nil then
        for i=1, #message.samples do
            if #sample_acc < 1024 * 16 then
                sample_acc = sample_acc .. string.char(message.samples[i])
            else
                if samples.push_back(sample_acc) == false then
                    pcPrint("HELp")
                end
                pcPrint("sent buf of len", #sample_acc)
                sample_acc = "" .. string.char(message.samples[i])
                os.queueEvent("new_samples")
            end
        end
    end
end

while true do
    parallel.waitForAny(wait_for_audio, read_ws, wait_for_close)
    if ws_closed then
        break
    end
end
