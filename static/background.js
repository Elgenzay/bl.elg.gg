class Background {
    constructor(canvas) {
        this.c_offset = 0.5;
        this.c_interval = 64;
        this.c_fill_speed = 0.02;
        this.c_pixel_brightness = 100;
        this.c_relative_pixel_size = 0.04;
        this.c_appearance_interval_primary = 2;
        this.c_appearance_interval_random = 6;
        this.c_random_brightness_min = 0.50;
        this.c_random_brightness_max = 0.75;

        this.canvas = canvas;

        if (!this.canvas) {
            console.error("Background canvas not found");
            return;
        }

        this.ctx = this.canvas.getContext("2d");
        window.ebackground = this;
        window.document.body.setAttribute("onresize", "window.ebackground.reset();");
        this.initialize();
        this.reset();
        this.frame = 0;
        setInterval(this.animate.bind(this), this.c_interval);
    }

    draw_map(map) {
        this.ctx.fillStyle = "#000";
        this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);

        for (let i in map) {
            this.draw_pixel(map[i]);
        }
    }

    animate() {
        this.frame++;

        if (this.frame % this.c_appearance_interval_random == 0) {
            let x = Math.ceil(this.pixels_x * Math.random() - (this.pixels_x / 2));
            let y = Math.ceil(this.pixels_y * Math.random() - (this.pixels_y / 2));

            this.map_random.push({
                x,
                y,
                "t": this.c_random_brightness_min + (Math.random() * this.rand_t_range),
                "f": false,
                "o": 0
            });
        }

        let map = [];

        for (let i in this.map_random) {
            if (this.map_random[i].f) {
                this.map_random[i].o -= this.c_fill_speed;

                if (this.map_random[i].o <= 0) {
                    delete this.map_random[i];
                    continue;
                }
            } else {
                this.map_random[i].o += this.c_fill_speed;

                if (this.map_random[i].o >= this.map_random[i].t) {
                    this.map_random[i].f = true;
                }
            }
            map.push(this.map_random[i]);
        }
        this.draw_map(map);
    }

    reset() {
        this.canvas.width = window.innerWidth;
        this.canvas.height = window.innerHeight;
        this.pixelsize = window.innerHeight * this.c_relative_pixel_size;
        this.pixels_x = (window.innerWidth / this.pixelsize);
        this.pixels_y = window.innerHeight / this.pixelsize;
        this.animate();
    }

    draw_pixel(pixel) {
        let c = Math.floor(this.c_pixel_brightness * pixel.o);
        this.ctx.fillStyle = "rgb(" + c + "," + c + "," + c + ")";

        this.ctx.fillRect(
            ((((this.pixels_x / 2) + pixel.x) * this.pixelsize) - (this.c_offset * this.pixelsize)),
            (((this.pixels_y / 2) + pixel.y) * this.pixelsize),
            this.pixelsize,
            this.pixelsize
        );

        this.ctx.fillStyle = "#000";

        this.ctx.fillRect(
            ((((this.pixels_x / 2) + pixel.x) * this.pixelsize) - (this.c_offset * this.pixelsize)) + 4,
            (((this.pixels_y / 2) + pixel.y) * this.pixelsize) + 4,
            this.pixelsize - 8,
            this.pixelsize - 8
        );
    }

    initialize() {
        this.map_random = [];
        this.rand_t_range = this.c_random_brightness_max - this.c_random_brightness_min;
    }
}
