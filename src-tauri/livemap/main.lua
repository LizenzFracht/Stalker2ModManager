-- S2MM Live-Map
-- Schreibt einmal pro Sekunde die Spielerposition nach %LOCALAPPDATA%\S2MM-Live\live.json.
-- Der Stalker 2 Mod Manager (S2MM) zeigt sie auf der interaktiven Karte im Browser an.

local UEHelpers = require("UEHelpers")

local dir = (os.getenv("LOCALAPPDATA") or ".") .. "\\S2MM-Live"
local target = dir .. "\\live.json"
local temp = dir .. "\\live.tmp"

local function writePosition()
    local pc = UEHelpers.GetPlayerController()
    if not pc or not pc:IsValid() then return end
    local pawn = pc.Pawn
    if not pawn or not pawn:IsValid() then return end
    local loc = pawn:K2_GetActorLocation()
    local rot = pc:GetControlRotation()
    local f = io.open(temp, "w")
    if not f then return end
    f:write(string.format('{"x":%.1f,"y":%.1f,"z":%.1f,"yaw":%.1f,"t":%d}', loc.X, loc.Y, loc.Z, rot.Yaw, os.time()))
    f:close()
    os.remove(target)
    os.rename(temp, target)
end

LoopAsync(1000, function()
    ExecuteInGameThread(function()
        pcall(writePosition)
    end)
    return false
end)

print("[S2MM LiveMap] aktiv – Position wird nach " .. target .. " geschrieben\n")
