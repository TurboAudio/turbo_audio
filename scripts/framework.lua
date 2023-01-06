Colors = {}

function Set_colors()
	local data = {}
	for i, value in pairs(Colors) do
		local index = ((i - 1) * 3)
		data[index + 1] = string.char(value.r)
		data[index + 2] = string.char(value.g)
		data[index + 3] = string.char(value.b)
	end
	Colors_bin = table.concat(data)
end

function Resize_Colors(len)
	if len ~= #Colors then
		Colors = {}
		for index = 1, len do
			Colors[index] = { r = 0, g = 0, b = 0 }
		end
	end
end
