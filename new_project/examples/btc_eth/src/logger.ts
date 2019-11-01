import minimist from "minimist";
import winston from "winston";

export interface CustomLogger extends winston.Logger {
    maker: winston.LeveledLogMethod;
    taker: winston.LeveledLogMethod;
    data: winston.LeveledLogMethod;
}

const colorizer = winston.format.colorize({
    all: true,
    colors: {
        error: "red",
        maker: "cyan",
        taker: "yellow",
        data: "grey",
    },
});

export function createLogger() {
    const level = minimist(process.argv.slice(2), {
        default: {
            loglevel: "info",
        },
    }).loglevel;

    if (level !== "info" && level !== "data") {
        throw new Error(
            `[error] Invalid log level: ${level}. Choose one from "info" or "data"`
        );
    }

    return winston.createLogger({
        levels: {
            error: 0,
            maker: 1,
            taker: 1,
            info: 1,
            data: 2,
        },
        format: winston.format.combine(
            winston.format.simple(),
            winston.format.splat(),
            winston.format.printf(msg => {
                const message =
                    msg.level === "data"
                        ? `  \u21B3[${msg.level}] ${msg.message}`
                        : `[${msg.level}] ${msg.message}`;

                return colorizer.colorize(msg.level, message);
            })
        ),
        transports: [new winston.transports.Console()],
        level,
    }) as CustomLogger;
}
