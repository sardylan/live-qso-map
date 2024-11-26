/*
 * Copyright (C) 2024 Luca Cireddu <sardylan@gmail.com>
 *
 * This program is free software: you can redistribute it and/or modify it under
 * the terms of the GNU General Public License as published by the Free Software
 * Foundation, version 3.
 *
 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * this program. If not, see <https://www.gnu.org/licenses/>.
 *
 */

use clap::{ArgAction, Parser};
use log::Level;

#[derive(Parser, Debug)]
#[command(version, author, about, long_about = None)]
pub struct Config {
    #[arg(
        short = 'l',
        long,
        action = ArgAction::Set,
        default_value = "WARN",
        help = "Log level",
        long_help = "Set the level of logging messages"
    )]
    pub log_level: Level,

    #[arg(
        short = 'H',
        long,
        action = ArgAction::Set,
        default_value = "::",
        help = "HTTP binding",
        long_help = "Binding address for the HTTP server"
    )]
    pub http_host: String,

    #[arg(
        short = 'P',
        long,
        action = ArgAction::Set,
        default_value = "8641",
        help = "HTTP port",
        long_help = "Port for the HTTP server"
    )]
    pub http_port: u16,

    #[arg(
        short = 'I',
        long,
        action = ArgAction::Set,
        default_value = "::",
        help = "QARTest binding",
        long_help = "Binding address for the QARTest UDP socket receiver"
    )]
    pub bind_host: String,

    #[arg(
        short = 'Q',
        long,
        action = ArgAction::Set,
        default_value = "12060",
        help = "QARTest port",
        long_help = "Port for the QARTest UDP socket receiver"
    )]
    pub bind_port: u16,

    #[arg(
        short = 'u',
        long,
        action = ArgAction::Set,
        required = true,
        help = "QRZ.com User",
        long_help = "Username for the QRZ.com XML APIs"
    )]
    pub qrzcom_user: String,

    #[arg(
        short = 'p',
        long,
        action = ArgAction::Set,
        required = true,
        help = "QRZ.com Password",
        long_help = "Password for the QRZ.com XML APIs"
    )]
    pub qrzcom_password: String,

    #[arg(
        short = 'a',
        long,
        action = ArgAction::Set,
        required = true,
        help = "Home Latitude",
        long_help = "Latitude of the home station"
    )]
    pub home_latitude: f64,

    #[arg(
        short = 'b',
        long,
        action = ArgAction::Set,
        required = true,
        help = "Home Longitude",
        long_help = "Longitude of the home station"
    )]
    pub home_longitude: f64,
}
