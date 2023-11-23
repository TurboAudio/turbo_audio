require("scripts.libs.framework")

SettingsSchema = {}

-- Local state
local tip_position = 0

function Tick()
	local new_r = math.floor(math.min(4 * Fft_Result:get_average_amplitude(0, 150), 255))
	local new_g = math.floor(math.min(15 * Fft_Result:get_average_amplitude(100, 1100), 255))
	local new_b = math.floor(math.min(20 * Fft_Result:get_average_amplitude(1000, 2000), 255))

    local red_bar_length = math.floor(#Colors * (new_r/255.0))
    for index = 1, red_bar_length do
        Colors[index].r = new_r
        Colors[index].g = new_g
        Colors[index].b = new_b
    end

	for index = red_bar_length + 1, #Colors do
		Colors[index].r = 0
		Colors[index].g = 0
		Colors[index].b = 0
	end


    local tip_length = math.floor(5 * (tip_position / #Colors)) + 2
    if tip_position < red_bar_length then
        tip_position = math.min(#Colors - tip_length, red_bar_length + 1)
    else
        local speed = 2
        tip_position = math.max(0, tip_position - speed)
    end


    if tip_position > 1 then
        for index = 0, tip_length do
            Colors[tip_position + index].r = 255
            Colors[tip_position + index].g = 255
            Colors[tip_position + index].b = 255
        end
    end
end
