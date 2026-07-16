/** @file
 *    @brief MAVLink comm protocol testsuite generated from cc_dialect.xml
 *    @see https://mavlink.io/en/
 */
#pragma once
#ifndef CC_DIALECT_TESTSUITE_H
#define CC_DIALECT_TESTSUITE_H

#ifdef __cplusplus
extern "C" {
#endif

#ifndef MAVLINK_TEST_ALL
#define MAVLINK_TEST_ALL
static void mavlink_test_common(uint8_t, uint8_t, mavlink_message_t *last_msg);
static void mavlink_test_cc_dialect(uint8_t, uint8_t, mavlink_message_t *last_msg);

static void mavlink_test_all(uint8_t system_id, uint8_t component_id, mavlink_message_t *last_msg)
{
    mavlink_test_common(system_id, component_id, last_msg);
    mavlink_test_cc_dialect(system_id, component_id, last_msg);
}
#endif

#include "../common/testsuite.h"


static void mavlink_test_cc_telemetry_state(uint8_t system_id, uint8_t component_id, mavlink_message_t *last_msg)
{
#ifdef MAVLINK_STATUS_FLAG_OUT_MAVLINK1
    mavlink_status_t *status = mavlink_get_channel_status(MAVLINK_COMM_0);
        if ((status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1) && MAVLINK_MSG_ID_CC_TELEMETRY_STATE >= 256) {
            return;
        }
#endif
    mavlink_message_t msg;
        uint8_t buffer[MAVLINK_MAX_PACKET_LEN];
        uint16_t i;
    mavlink_cc_telemetry_state_t packet_in = {
        93372036854775807ULL,963497880,963498088,963498296,963498504,{ 185.0, 186.0, 187.0, 188.0 },{ 297.0, 298.0, 299.0 },{ 381.0, 382.0, 383.0 },{ 465.0, 466.0, 467.0 },549.0,245,56,123,190,1,68
    };
    mavlink_cc_telemetry_state_t packet1, packet2;
        memset(&packet1, 0, sizeof(packet1));
        packet1.fc_timestamp_us = packet_in.fc_timestamp_us;
        packet1.sequence = packet_in.sequence;
        packet1.px4_boot_id = packet_in.px4_boot_id;
        packet1.mission_id = packet_in.mission_id;
        packet1.failsafe_flags = packet_in.failsafe_flags;
        packet1.heading = packet_in.heading;
        packet1.nav_state = packet_in.nav_state;
        packet1.arming_state = packet_in.arming_state;
        packet1.vehicle_type = packet_in.vehicle_type;
        packet1.estimator_valid = packet_in.estimator_valid;
        packet1.control_mode_flags = packet_in.control_mode_flags;
        packet1.schema_version = packet_in.schema_version;
        
        mav_array_memcpy(packet1.q, packet_in.q, sizeof(float)*4);
        mav_array_memcpy(packet1.angular_velocity, packet_in.angular_velocity, sizeof(float)*3);
        mav_array_memcpy(packet1.position_ned, packet_in.position_ned, sizeof(float)*3);
        mav_array_memcpy(packet1.velocity_ned, packet_in.velocity_ned, sizeof(float)*3);
        
#ifdef MAVLINK_STATUS_FLAG_OUT_MAVLINK1
        if (status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1) {
           // cope with extensions
           memset(MAVLINK_MSG_ID_CC_TELEMETRY_STATE_MIN_LEN + (char *)&packet1, 0, sizeof(packet1)-MAVLINK_MSG_ID_CC_TELEMETRY_STATE_MIN_LEN);
        }
#endif
        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_telemetry_state_encode(system_id, component_id, &msg, &packet1);
    mavlink_msg_cc_telemetry_state_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_telemetry_state_pack(system_id, component_id, &msg , packet1.fc_timestamp_us , packet1.sequence , packet1.px4_boot_id , packet1.mission_id , packet1.failsafe_flags , packet1.q , packet1.angular_velocity , packet1.position_ned , packet1.velocity_ned , packet1.heading , packet1.nav_state , packet1.arming_state , packet1.vehicle_type , packet1.estimator_valid , packet1.control_mode_flags , packet1.schema_version );
    mavlink_msg_cc_telemetry_state_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_telemetry_state_pack_chan(system_id, component_id, MAVLINK_COMM_0, &msg , packet1.fc_timestamp_us , packet1.sequence , packet1.px4_boot_id , packet1.mission_id , packet1.failsafe_flags , packet1.q , packet1.angular_velocity , packet1.position_ned , packet1.velocity_ned , packet1.heading , packet1.nav_state , packet1.arming_state , packet1.vehicle_type , packet1.estimator_valid , packet1.control_mode_flags , packet1.schema_version );
    mavlink_msg_cc_telemetry_state_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
        mavlink_msg_to_send_buffer(buffer, &msg);
        for (i=0; i<mavlink_msg_get_send_buffer_length(&msg); i++) {
            comm_send_ch(MAVLINK_COMM_0, buffer[i]);
        }
    mavlink_msg_cc_telemetry_state_decode(last_msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);
        
        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_telemetry_state_send(MAVLINK_COMM_1 , packet1.fc_timestamp_us , packet1.sequence , packet1.px4_boot_id , packet1.mission_id , packet1.failsafe_flags , packet1.q , packet1.angular_velocity , packet1.position_ned , packet1.velocity_ned , packet1.heading , packet1.nav_state , packet1.arming_state , packet1.vehicle_type , packet1.estimator_valid , packet1.control_mode_flags , packet1.schema_version );
    mavlink_msg_cc_telemetry_state_decode(last_msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

#ifdef MAVLINK_HAVE_GET_MESSAGE_INFO
    MAVLINK_ASSERT(mavlink_get_message_info_by_name("CC_TELEMETRY_STATE") != NULL);
    MAVLINK_ASSERT(mavlink_get_message_info_by_id(MAVLINK_MSG_ID_CC_TELEMETRY_STATE) != NULL);
#endif
}

static void mavlink_test_cc_telemetry_imu(uint8_t system_id, uint8_t component_id, mavlink_message_t *last_msg)
{
#ifdef MAVLINK_STATUS_FLAG_OUT_MAVLINK1
    mavlink_status_t *status = mavlink_get_channel_status(MAVLINK_COMM_0);
        if ((status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1) && MAVLINK_MSG_ID_CC_TELEMETRY_IMU >= 256) {
            return;
        }
#endif
    mavlink_message_t msg;
        uint8_t buffer[MAVLINK_MAX_PACKET_LEN];
        uint16_t i;
    mavlink_cc_telemetry_imu_t packet_in = {
        93372036854775807ULL,963497880,963498088,{ 129.0, 130.0, 131.0 },{ 213.0, 214.0, 215.0 },{ 297.0, 298.0, 299.0 },{ 381.0, 382.0, 383.0 },{ 465.0, 466.0, 467.0 },549.0,245
    };
    mavlink_cc_telemetry_imu_t packet1, packet2;
        memset(&packet1, 0, sizeof(packet1));
        packet1.fc_timestamp_us = packet_in.fc_timestamp_us;
        packet1.sequence = packet_in.sequence;
        packet1.clipping_count = packet_in.clipping_count;
        packet1.temperature = packet_in.temperature;
        packet1.schema_version = packet_in.schema_version;
        
        mav_array_memcpy(packet1.accel, packet_in.accel, sizeof(float)*3);
        mav_array_memcpy(packet1.gyro, packet_in.gyro, sizeof(float)*3);
        mav_array_memcpy(packet1.delta_angle, packet_in.delta_angle, sizeof(float)*3);
        mav_array_memcpy(packet1.delta_velocity, packet_in.delta_velocity, sizeof(float)*3);
        mav_array_memcpy(packet1.vibration_metric, packet_in.vibration_metric, sizeof(float)*3);
        
#ifdef MAVLINK_STATUS_FLAG_OUT_MAVLINK1
        if (status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1) {
           // cope with extensions
           memset(MAVLINK_MSG_ID_CC_TELEMETRY_IMU_MIN_LEN + (char *)&packet1, 0, sizeof(packet1)-MAVLINK_MSG_ID_CC_TELEMETRY_IMU_MIN_LEN);
        }
#endif
        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_telemetry_imu_encode(system_id, component_id, &msg, &packet1);
    mavlink_msg_cc_telemetry_imu_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_telemetry_imu_pack(system_id, component_id, &msg , packet1.fc_timestamp_us , packet1.sequence , packet1.clipping_count , packet1.accel , packet1.gyro , packet1.delta_angle , packet1.delta_velocity , packet1.vibration_metric , packet1.temperature , packet1.schema_version );
    mavlink_msg_cc_telemetry_imu_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_telemetry_imu_pack_chan(system_id, component_id, MAVLINK_COMM_0, &msg , packet1.fc_timestamp_us , packet1.sequence , packet1.clipping_count , packet1.accel , packet1.gyro , packet1.delta_angle , packet1.delta_velocity , packet1.vibration_metric , packet1.temperature , packet1.schema_version );
    mavlink_msg_cc_telemetry_imu_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
        mavlink_msg_to_send_buffer(buffer, &msg);
        for (i=0; i<mavlink_msg_get_send_buffer_length(&msg); i++) {
            comm_send_ch(MAVLINK_COMM_0, buffer[i]);
        }
    mavlink_msg_cc_telemetry_imu_decode(last_msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);
        
        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_telemetry_imu_send(MAVLINK_COMM_1 , packet1.fc_timestamp_us , packet1.sequence , packet1.clipping_count , packet1.accel , packet1.gyro , packet1.delta_angle , packet1.delta_velocity , packet1.vibration_metric , packet1.temperature , packet1.schema_version );
    mavlink_msg_cc_telemetry_imu_decode(last_msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

#ifdef MAVLINK_HAVE_GET_MESSAGE_INFO
    MAVLINK_ASSERT(mavlink_get_message_info_by_name("CC_TELEMETRY_IMU") != NULL);
    MAVLINK_ASSERT(mavlink_get_message_info_by_id(MAVLINK_MSG_ID_CC_TELEMETRY_IMU) != NULL);
#endif
}

static void mavlink_test_cc_telemetry_power(uint8_t system_id, uint8_t component_id, mavlink_message_t *last_msg)
{
#ifdef MAVLINK_STATUS_FLAG_OUT_MAVLINK1
    mavlink_status_t *status = mavlink_get_channel_status(MAVLINK_COMM_0);
        if ((status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1) && MAVLINK_MSG_ID_CC_TELEMETRY_POWER >= 256) {
            return;
        }
#endif
    mavlink_message_t msg;
        uint8_t buffer[MAVLINK_MAX_PACKET_LEN];
        uint16_t i;
    mavlink_cc_telemetry_power_t packet_in = {
        93372036854775807ULL,963497880,101.0,129.0,157.0,185.0,213.0,241.0,113,180,247,58
    };
    mavlink_cc_telemetry_power_t packet1, packet2;
        memset(&packet1, 0, sizeof(packet1));
        packet1.fc_timestamp_us = packet_in.fc_timestamp_us;
        packet1.sequence = packet_in.sequence;
        packet1.voltage = packet_in.voltage;
        packet1.current = packet_in.current;
        packet1.power = packet_in.power;
        packet1.consumed_mah = packet_in.consumed_mah;
        packet1.remaining = packet_in.remaining;
        packet1.temperature = packet_in.temperature;
        packet1.cell_count = packet_in.cell_count;
        packet1.warning = packet_in.warning;
        packet1.connected = packet_in.connected;
        packet1.schema_version = packet_in.schema_version;
        
        
#ifdef MAVLINK_STATUS_FLAG_OUT_MAVLINK1
        if (status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1) {
           // cope with extensions
           memset(MAVLINK_MSG_ID_CC_TELEMETRY_POWER_MIN_LEN + (char *)&packet1, 0, sizeof(packet1)-MAVLINK_MSG_ID_CC_TELEMETRY_POWER_MIN_LEN);
        }
#endif
        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_telemetry_power_encode(system_id, component_id, &msg, &packet1);
    mavlink_msg_cc_telemetry_power_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_telemetry_power_pack(system_id, component_id, &msg , packet1.fc_timestamp_us , packet1.sequence , packet1.voltage , packet1.current , packet1.power , packet1.consumed_mah , packet1.remaining , packet1.temperature , packet1.cell_count , packet1.warning , packet1.connected , packet1.schema_version );
    mavlink_msg_cc_telemetry_power_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_telemetry_power_pack_chan(system_id, component_id, MAVLINK_COMM_0, &msg , packet1.fc_timestamp_us , packet1.sequence , packet1.voltage , packet1.current , packet1.power , packet1.consumed_mah , packet1.remaining , packet1.temperature , packet1.cell_count , packet1.warning , packet1.connected , packet1.schema_version );
    mavlink_msg_cc_telemetry_power_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
        mavlink_msg_to_send_buffer(buffer, &msg);
        for (i=0; i<mavlink_msg_get_send_buffer_length(&msg); i++) {
            comm_send_ch(MAVLINK_COMM_0, buffer[i]);
        }
    mavlink_msg_cc_telemetry_power_decode(last_msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);
        
        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_telemetry_power_send(MAVLINK_COMM_1 , packet1.fc_timestamp_us , packet1.sequence , packet1.voltage , packet1.current , packet1.power , packet1.consumed_mah , packet1.remaining , packet1.temperature , packet1.cell_count , packet1.warning , packet1.connected , packet1.schema_version );
    mavlink_msg_cc_telemetry_power_decode(last_msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

#ifdef MAVLINK_HAVE_GET_MESSAGE_INFO
    MAVLINK_ASSERT(mavlink_get_message_info_by_name("CC_TELEMETRY_POWER") != NULL);
    MAVLINK_ASSERT(mavlink_get_message_info_by_id(MAVLINK_MSG_ID_CC_TELEMETRY_POWER) != NULL);
#endif
}

static void mavlink_test_cc_telemetry_gps(uint8_t system_id, uint8_t component_id, mavlink_message_t *last_msg)
{
#ifdef MAVLINK_STATUS_FLAG_OUT_MAVLINK1
    mavlink_status_t *status = mavlink_get_channel_status(MAVLINK_COMM_0);
        if ((status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1) && MAVLINK_MSG_ID_CC_TELEMETRY_GPS >= 256) {
            return;
        }
#endif
    mavlink_message_t msg;
        uint8_t buffer[MAVLINK_MAX_PACKET_LEN];
        uint16_t i;
    mavlink_cc_telemetry_gps_t packet_in = {
        93372036854775807ULL,963497880,963498088,963498296,963498504,185.0,213.0,241.0,269.0,19315,19419,137,204,15
    };
    mavlink_cc_telemetry_gps_t packet1, packet2;
        memset(&packet1, 0, sizeof(packet1));
        packet1.fc_timestamp_us = packet_in.fc_timestamp_us;
        packet1.sequence = packet_in.sequence;
        packet1.lat = packet_in.lat;
        packet1.lon = packet_in.lon;
        packet1.alt = packet_in.alt;
        packet1.eph = packet_in.eph;
        packet1.epv = packet_in.epv;
        packet1.ground_speed = packet_in.ground_speed;
        packet1.heading = packet_in.heading;
        packet1.noise_per_ms = packet_in.noise_per_ms;
        packet1.jamming_indicator = packet_in.jamming_indicator;
        packet1.fix_type = packet_in.fix_type;
        packet1.satellites_used = packet_in.satellites_used;
        packet1.schema_version = packet_in.schema_version;
        
        
#ifdef MAVLINK_STATUS_FLAG_OUT_MAVLINK1
        if (status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1) {
           // cope with extensions
           memset(MAVLINK_MSG_ID_CC_TELEMETRY_GPS_MIN_LEN + (char *)&packet1, 0, sizeof(packet1)-MAVLINK_MSG_ID_CC_TELEMETRY_GPS_MIN_LEN);
        }
#endif
        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_telemetry_gps_encode(system_id, component_id, &msg, &packet1);
    mavlink_msg_cc_telemetry_gps_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_telemetry_gps_pack(system_id, component_id, &msg , packet1.fc_timestamp_us , packet1.sequence , packet1.lat , packet1.lon , packet1.alt , packet1.eph , packet1.epv , packet1.ground_speed , packet1.heading , packet1.noise_per_ms , packet1.jamming_indicator , packet1.fix_type , packet1.satellites_used , packet1.schema_version );
    mavlink_msg_cc_telemetry_gps_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_telemetry_gps_pack_chan(system_id, component_id, MAVLINK_COMM_0, &msg , packet1.fc_timestamp_us , packet1.sequence , packet1.lat , packet1.lon , packet1.alt , packet1.eph , packet1.epv , packet1.ground_speed , packet1.heading , packet1.noise_per_ms , packet1.jamming_indicator , packet1.fix_type , packet1.satellites_used , packet1.schema_version );
    mavlink_msg_cc_telemetry_gps_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
        mavlink_msg_to_send_buffer(buffer, &msg);
        for (i=0; i<mavlink_msg_get_send_buffer_length(&msg); i++) {
            comm_send_ch(MAVLINK_COMM_0, buffer[i]);
        }
    mavlink_msg_cc_telemetry_gps_decode(last_msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);
        
        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_telemetry_gps_send(MAVLINK_COMM_1 , packet1.fc_timestamp_us , packet1.sequence , packet1.lat , packet1.lon , packet1.alt , packet1.eph , packet1.epv , packet1.ground_speed , packet1.heading , packet1.noise_per_ms , packet1.jamming_indicator , packet1.fix_type , packet1.satellites_used , packet1.schema_version );
    mavlink_msg_cc_telemetry_gps_decode(last_msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

#ifdef MAVLINK_HAVE_GET_MESSAGE_INFO
    MAVLINK_ASSERT(mavlink_get_message_info_by_name("CC_TELEMETRY_GPS") != NULL);
    MAVLINK_ASSERT(mavlink_get_message_info_by_id(MAVLINK_MSG_ID_CC_TELEMETRY_GPS) != NULL);
#endif
}

static void mavlink_test_cc_telemetry_estimator(uint8_t system_id, uint8_t component_id, mavlink_message_t *last_msg)
{
#ifdef MAVLINK_STATUS_FLAG_OUT_MAVLINK1
    mavlink_status_t *status = mavlink_get_channel_status(MAVLINK_COMM_0);
        if ((status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1) && MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR >= 256) {
            return;
        }
#endif
    mavlink_message_t msg;
        uint8_t buffer[MAVLINK_MAX_PACKET_LEN];
        uint16_t i;
    mavlink_cc_telemetry_estimator_t packet_in = {
        93372036854775807ULL,963497880,963498088,129.0,157.0,185.0,213.0,241.0,19107,19211,125
    };
    mavlink_cc_telemetry_estimator_t packet1, packet2;
        memset(&packet1, 0, sizeof(packet1));
        packet1.fc_timestamp_us = packet_in.fc_timestamp_us;
        packet1.sequence = packet_in.sequence;
        packet1.status_flags = packet_in.status_flags;
        packet1.velocity_test_ratio = packet_in.velocity_test_ratio;
        packet1.position_test_ratio = packet_in.position_test_ratio;
        packet1.height_test_ratio = packet_in.height_test_ratio;
        packet1.mag_test_ratio = packet_in.mag_test_ratio;
        packet1.airspeed_test_ratio = packet_in.airspeed_test_ratio;
        packet1.innovation_check_flags = packet_in.innovation_check_flags;
        packet1.solution_status_flags = packet_in.solution_status_flags;
        packet1.schema_version = packet_in.schema_version;
        
        
#ifdef MAVLINK_STATUS_FLAG_OUT_MAVLINK1
        if (status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1) {
           // cope with extensions
           memset(MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_MIN_LEN + (char *)&packet1, 0, sizeof(packet1)-MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR_MIN_LEN);
        }
#endif
        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_telemetry_estimator_encode(system_id, component_id, &msg, &packet1);
    mavlink_msg_cc_telemetry_estimator_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_telemetry_estimator_pack(system_id, component_id, &msg , packet1.fc_timestamp_us , packet1.sequence , packet1.status_flags , packet1.innovation_check_flags , packet1.solution_status_flags , packet1.velocity_test_ratio , packet1.position_test_ratio , packet1.height_test_ratio , packet1.mag_test_ratio , packet1.airspeed_test_ratio , packet1.schema_version );
    mavlink_msg_cc_telemetry_estimator_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_telemetry_estimator_pack_chan(system_id, component_id, MAVLINK_COMM_0, &msg , packet1.fc_timestamp_us , packet1.sequence , packet1.status_flags , packet1.innovation_check_flags , packet1.solution_status_flags , packet1.velocity_test_ratio , packet1.position_test_ratio , packet1.height_test_ratio , packet1.mag_test_ratio , packet1.airspeed_test_ratio , packet1.schema_version );
    mavlink_msg_cc_telemetry_estimator_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
        mavlink_msg_to_send_buffer(buffer, &msg);
        for (i=0; i<mavlink_msg_get_send_buffer_length(&msg); i++) {
            comm_send_ch(MAVLINK_COMM_0, buffer[i]);
        }
    mavlink_msg_cc_telemetry_estimator_decode(last_msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);
        
        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_telemetry_estimator_send(MAVLINK_COMM_1 , packet1.fc_timestamp_us , packet1.sequence , packet1.status_flags , packet1.innovation_check_flags , packet1.solution_status_flags , packet1.velocity_test_ratio , packet1.position_test_ratio , packet1.height_test_ratio , packet1.mag_test_ratio , packet1.airspeed_test_ratio , packet1.schema_version );
    mavlink_msg_cc_telemetry_estimator_decode(last_msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

#ifdef MAVLINK_HAVE_GET_MESSAGE_INFO
    MAVLINK_ASSERT(mavlink_get_message_info_by_name("CC_TELEMETRY_ESTIMATOR") != NULL);
    MAVLINK_ASSERT(mavlink_get_message_info_by_id(MAVLINK_MSG_ID_CC_TELEMETRY_ESTIMATOR) != NULL);
#endif
}

static void mavlink_test_cc_telemetry_actuator(uint8_t system_id, uint8_t component_id, mavlink_message_t *last_msg)
{
#ifdef MAVLINK_STATUS_FLAG_OUT_MAVLINK1
    mavlink_status_t *status = mavlink_get_channel_status(MAVLINK_COMM_0);
        if ((status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1) && MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR >= 256) {
            return;
        }
#endif
    mavlink_message_t msg;
        uint8_t buffer[MAVLINK_MAX_PACKET_LEN];
        uint16_t i;
    mavlink_cc_telemetry_actuator_t packet_in = {
        93372036854775807ULL,963497880,{ 101.0, 102.0, 103.0, 104.0, 105.0, 106.0, 107.0, 108.0 },137,204
    };
    mavlink_cc_telemetry_actuator_t packet1, packet2;
        memset(&packet1, 0, sizeof(packet1));
        packet1.fc_timestamp_us = packet_in.fc_timestamp_us;
        packet1.sequence = packet_in.sequence;
        packet1.motor_count = packet_in.motor_count;
        packet1.schema_version = packet_in.schema_version;
        
        mav_array_memcpy(packet1.actuator_output, packet_in.actuator_output, sizeof(float)*8);
        
#ifdef MAVLINK_STATUS_FLAG_OUT_MAVLINK1
        if (status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1) {
           // cope with extensions
           memset(MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_MIN_LEN + (char *)&packet1, 0, sizeof(packet1)-MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR_MIN_LEN);
        }
#endif
        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_telemetry_actuator_encode(system_id, component_id, &msg, &packet1);
    mavlink_msg_cc_telemetry_actuator_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_telemetry_actuator_pack(system_id, component_id, &msg , packet1.fc_timestamp_us , packet1.sequence , packet1.actuator_output , packet1.motor_count , packet1.schema_version );
    mavlink_msg_cc_telemetry_actuator_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_telemetry_actuator_pack_chan(system_id, component_id, MAVLINK_COMM_0, &msg , packet1.fc_timestamp_us , packet1.sequence , packet1.actuator_output , packet1.motor_count , packet1.schema_version );
    mavlink_msg_cc_telemetry_actuator_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
        mavlink_msg_to_send_buffer(buffer, &msg);
        for (i=0; i<mavlink_msg_get_send_buffer_length(&msg); i++) {
            comm_send_ch(MAVLINK_COMM_0, buffer[i]);
        }
    mavlink_msg_cc_telemetry_actuator_decode(last_msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);
        
        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_telemetry_actuator_send(MAVLINK_COMM_1 , packet1.fc_timestamp_us , packet1.sequence , packet1.actuator_output , packet1.motor_count , packet1.schema_version );
    mavlink_msg_cc_telemetry_actuator_decode(last_msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

#ifdef MAVLINK_HAVE_GET_MESSAGE_INFO
    MAVLINK_ASSERT(mavlink_get_message_info_by_name("CC_TELEMETRY_ACTUATOR") != NULL);
    MAVLINK_ASSERT(mavlink_get_message_info_by_id(MAVLINK_MSG_ID_CC_TELEMETRY_ACTUATOR) != NULL);
#endif
}

static void mavlink_test_cc_event(uint8_t system_id, uint8_t component_id, mavlink_message_t *last_msg)
{
#ifdef MAVLINK_STATUS_FLAG_OUT_MAVLINK1
    mavlink_status_t *status = mavlink_get_channel_status(MAVLINK_COMM_0);
        if ((status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1) && MAVLINK_MSG_ID_CC_EVENT >= 256) {
            return;
        }
#endif
    mavlink_message_t msg;
        uint8_t buffer[MAVLINK_MAX_PACKET_LEN];
        uint16_t i;
    mavlink_cc_event_t packet_in = {
        93372036854775807ULL,963497880,963498088,963498296,963498504,77,144,211
    };
    mavlink_cc_event_t packet1, packet2;
        memset(&packet1, 0, sizeof(packet1));
        packet1.fc_timestamp_us = packet_in.fc_timestamp_us;
        packet1.sequence = packet_in.sequence;
        packet1.event_id = packet_in.event_id;
        packet1.argument0 = packet_in.argument0;
        packet1.argument1 = packet_in.argument1;
        packet1.severity = packet_in.severity;
        packet1.subsystem = packet_in.subsystem;
        packet1.schema_version = packet_in.schema_version;
        
        
#ifdef MAVLINK_STATUS_FLAG_OUT_MAVLINK1
        if (status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1) {
           // cope with extensions
           memset(MAVLINK_MSG_ID_CC_EVENT_MIN_LEN + (char *)&packet1, 0, sizeof(packet1)-MAVLINK_MSG_ID_CC_EVENT_MIN_LEN);
        }
#endif
        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_event_encode(system_id, component_id, &msg, &packet1);
    mavlink_msg_cc_event_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_event_pack(system_id, component_id, &msg , packet1.fc_timestamp_us , packet1.sequence , packet1.event_id , packet1.argument0 , packet1.argument1 , packet1.severity , packet1.subsystem , packet1.schema_version );
    mavlink_msg_cc_event_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_event_pack_chan(system_id, component_id, MAVLINK_COMM_0, &msg , packet1.fc_timestamp_us , packet1.sequence , packet1.event_id , packet1.argument0 , packet1.argument1 , packet1.severity , packet1.subsystem , packet1.schema_version );
    mavlink_msg_cc_event_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
        mavlink_msg_to_send_buffer(buffer, &msg);
        for (i=0; i<mavlink_msg_get_send_buffer_length(&msg); i++) {
            comm_send_ch(MAVLINK_COMM_0, buffer[i]);
        }
    mavlink_msg_cc_event_decode(last_msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);
        
        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_event_send(MAVLINK_COMM_1 , packet1.fc_timestamp_us , packet1.sequence , packet1.event_id , packet1.argument0 , packet1.argument1 , packet1.severity , packet1.subsystem , packet1.schema_version );
    mavlink_msg_cc_event_decode(last_msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

#ifdef MAVLINK_HAVE_GET_MESSAGE_INFO
    MAVLINK_ASSERT(mavlink_get_message_info_by_name("CC_EVENT") != NULL);
    MAVLINK_ASSERT(mavlink_get_message_info_by_id(MAVLINK_MSG_ID_CC_EVENT) != NULL);
#endif
}

static void mavlink_test_cc_safety_status(uint8_t system_id, uint8_t component_id, mavlink_message_t *last_msg)
{
#ifdef MAVLINK_STATUS_FLAG_OUT_MAVLINK1
    mavlink_status_t *status = mavlink_get_channel_status(MAVLINK_COMM_0);
        if ((status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1) && MAVLINK_MSG_ID_CC_SAFETY_STATUS >= 256) {
            return;
        }
#endif
    mavlink_message_t msg;
        uint8_t buffer[MAVLINK_MAX_PACKET_LEN];
        uint16_t i;
    mavlink_cc_safety_status_t packet_in = {
        93372036854775807ULL,963497880,963498088,963498296,963498504,77,144,211,22
    };
    mavlink_cc_safety_status_t packet1, packet2;
        memset(&packet1, 0, sizeof(packet1));
        packet1.fc_timestamp_us = packet_in.fc_timestamp_us;
        packet1.last_report_sequence = packet_in.last_report_sequence;
        packet1.active_health_flags = packet_in.active_health_flags;
        packet1.report_age_ms = packet_in.report_age_ms;
        packet1.missed_reports = packet_in.missed_reports;
        packet1.companion_state = packet_in.companion_state;
        packet1.action_taken = packet_in.action_taken;
        packet1.reject_reason = packet_in.reject_reason;
        packet1.schema_version = packet_in.schema_version;
        
        
#ifdef MAVLINK_STATUS_FLAG_OUT_MAVLINK1
        if (status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1) {
           // cope with extensions
           memset(MAVLINK_MSG_ID_CC_SAFETY_STATUS_MIN_LEN + (char *)&packet1, 0, sizeof(packet1)-MAVLINK_MSG_ID_CC_SAFETY_STATUS_MIN_LEN);
        }
#endif
        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_safety_status_encode(system_id, component_id, &msg, &packet1);
    mavlink_msg_cc_safety_status_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_safety_status_pack(system_id, component_id, &msg , packet1.fc_timestamp_us , packet1.last_report_sequence , packet1.active_health_flags , packet1.report_age_ms , packet1.missed_reports , packet1.companion_state , packet1.action_taken , packet1.reject_reason , packet1.schema_version );
    mavlink_msg_cc_safety_status_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_safety_status_pack_chan(system_id, component_id, MAVLINK_COMM_0, &msg , packet1.fc_timestamp_us , packet1.last_report_sequence , packet1.active_health_flags , packet1.report_age_ms , packet1.missed_reports , packet1.companion_state , packet1.action_taken , packet1.reject_reason , packet1.schema_version );
    mavlink_msg_cc_safety_status_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
        mavlink_msg_to_send_buffer(buffer, &msg);
        for (i=0; i<mavlink_msg_get_send_buffer_length(&msg); i++) {
            comm_send_ch(MAVLINK_COMM_0, buffer[i]);
        }
    mavlink_msg_cc_safety_status_decode(last_msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);
        
        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_safety_status_send(MAVLINK_COMM_1 , packet1.fc_timestamp_us , packet1.last_report_sequence , packet1.active_health_flags , packet1.report_age_ms , packet1.missed_reports , packet1.companion_state , packet1.action_taken , packet1.reject_reason , packet1.schema_version );
    mavlink_msg_cc_safety_status_decode(last_msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

#ifdef MAVLINK_HAVE_GET_MESSAGE_INFO
    MAVLINK_ASSERT(mavlink_get_message_info_by_name("CC_SAFETY_STATUS") != NULL);
    MAVLINK_ASSERT(mavlink_get_message_info_by_id(MAVLINK_MSG_ID_CC_SAFETY_STATUS) != NULL);
#endif
}

static void mavlink_test_cc_health_report(uint8_t system_id, uint8_t component_id, mavlink_message_t *last_msg)
{
#ifdef MAVLINK_STATUS_FLAG_OUT_MAVLINK1
    mavlink_status_t *status = mavlink_get_channel_status(MAVLINK_COMM_0);
        if ((status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1) && MAVLINK_MSG_ID_CC_HEALTH_REPORT >= 256) {
            return;
        }
#endif
    mavlink_message_t msg;
        uint8_t buffer[MAVLINK_MAX_PACKET_LEN];
        uint16_t i;
    mavlink_cc_health_report_t packet_in = {
        93372036854775807ULL,963497880,963498088,963498296,963498504,18483,18587,18691,18795,18899,235,46,113,180
    };
    mavlink_cc_health_report_t packet1, packet2;
        memset(&packet1, 0, sizeof(packet1));
        packet1.companion_timestamp_us = packet_in.companion_timestamp_us;
        packet1.sequence = packet_in.sequence;
        packet1.mission_id = packet_in.mission_id;
        packet1.companion_boot_id = packet_in.companion_boot_id;
        packet1.health_flags = packet_in.health_flags;
        packet1.detail_code = packet_in.detail_code;
        packet1.link_rtt_ms = packet_in.link_rtt_ms;
        packet1.telemetry_age_ms = packet_in.telemetry_age_ms;
        packet1.companion_loop_ms = packet_in.companion_loop_ms;
        packet1.dropped_rx_count = packet_in.dropped_rx_count;
        packet1.severity = packet_in.severity;
        packet1.recommended_action = packet_in.recommended_action;
        packet1.confidence_percent = packet_in.confidence_percent;
        packet1.schema_version = packet_in.schema_version;
        
        
#ifdef MAVLINK_STATUS_FLAG_OUT_MAVLINK1
        if (status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1) {
           // cope with extensions
           memset(MAVLINK_MSG_ID_CC_HEALTH_REPORT_MIN_LEN + (char *)&packet1, 0, sizeof(packet1)-MAVLINK_MSG_ID_CC_HEALTH_REPORT_MIN_LEN);
        }
#endif
        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_health_report_encode(system_id, component_id, &msg, &packet1);
    mavlink_msg_cc_health_report_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_health_report_pack(system_id, component_id, &msg , packet1.companion_timestamp_us , packet1.sequence , packet1.mission_id , packet1.companion_boot_id , packet1.health_flags , packet1.detail_code , packet1.link_rtt_ms , packet1.telemetry_age_ms , packet1.companion_loop_ms , packet1.dropped_rx_count , packet1.severity , packet1.recommended_action , packet1.confidence_percent , packet1.schema_version );
    mavlink_msg_cc_health_report_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_health_report_pack_chan(system_id, component_id, MAVLINK_COMM_0, &msg , packet1.companion_timestamp_us , packet1.sequence , packet1.mission_id , packet1.companion_boot_id , packet1.health_flags , packet1.detail_code , packet1.link_rtt_ms , packet1.telemetry_age_ms , packet1.companion_loop_ms , packet1.dropped_rx_count , packet1.severity , packet1.recommended_action , packet1.confidence_percent , packet1.schema_version );
    mavlink_msg_cc_health_report_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
        mavlink_msg_to_send_buffer(buffer, &msg);
        for (i=0; i<mavlink_msg_get_send_buffer_length(&msg); i++) {
            comm_send_ch(MAVLINK_COMM_0, buffer[i]);
        }
    mavlink_msg_cc_health_report_decode(last_msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);
        
        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_health_report_send(MAVLINK_COMM_1 , packet1.companion_timestamp_us , packet1.sequence , packet1.mission_id , packet1.companion_boot_id , packet1.health_flags , packet1.detail_code , packet1.link_rtt_ms , packet1.telemetry_age_ms , packet1.companion_loop_ms , packet1.dropped_rx_count , packet1.severity , packet1.recommended_action , packet1.confidence_percent , packet1.schema_version );
    mavlink_msg_cc_health_report_decode(last_msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

#ifdef MAVLINK_HAVE_GET_MESSAGE_INFO
    MAVLINK_ASSERT(mavlink_get_message_info_by_name("CC_HEALTH_REPORT") != NULL);
    MAVLINK_ASSERT(mavlink_get_message_info_by_id(MAVLINK_MSG_ID_CC_HEALTH_REPORT) != NULL);
#endif
}

static void mavlink_test_cc_ai_diagnostic(uint8_t system_id, uint8_t component_id, mavlink_message_t *last_msg)
{
#ifdef MAVLINK_STATUS_FLAG_OUT_MAVLINK1
    mavlink_status_t *status = mavlink_get_channel_status(MAVLINK_COMM_0);
        if ((status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1) && MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC >= 256) {
            return;
        }
#endif
    mavlink_message_t msg;
        uint8_t buffer[MAVLINK_MAX_PACKET_LEN];
        uint16_t i;
    mavlink_cc_ai_diagnostic_t packet_in = {
        93372036854775807ULL,963497880,101.0,129.0,18275,199,10,77,144
    };
    mavlink_cc_ai_diagnostic_t packet1, packet2;
        memset(&packet1, 0, sizeof(packet1));
        packet1.companion_timestamp_us = packet_in.companion_timestamp_us;
        packet1.sequence = packet_in.sequence;
        packet1.value = packet_in.value;
        packet1.limit = packet_in.limit;
        packet1.detail_code = packet_in.detail_code;
        packet1.subsystem = packet_in.subsystem;
        packet1.severity = packet_in.severity;
        packet1.confidence_percent = packet_in.confidence_percent;
        packet1.schema_version = packet_in.schema_version;
        
        
#ifdef MAVLINK_STATUS_FLAG_OUT_MAVLINK1
        if (status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1) {
           // cope with extensions
           memset(MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_MIN_LEN + (char *)&packet1, 0, sizeof(packet1)-MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC_MIN_LEN);
        }
#endif
        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_ai_diagnostic_encode(system_id, component_id, &msg, &packet1);
    mavlink_msg_cc_ai_diagnostic_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_ai_diagnostic_pack(system_id, component_id, &msg , packet1.companion_timestamp_us , packet1.sequence , packet1.value , packet1.limit , packet1.detail_code , packet1.subsystem , packet1.severity , packet1.confidence_percent , packet1.schema_version );
    mavlink_msg_cc_ai_diagnostic_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_ai_diagnostic_pack_chan(system_id, component_id, MAVLINK_COMM_0, &msg , packet1.companion_timestamp_us , packet1.sequence , packet1.value , packet1.limit , packet1.detail_code , packet1.subsystem , packet1.severity , packet1.confidence_percent , packet1.schema_version );
    mavlink_msg_cc_ai_diagnostic_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
        mavlink_msg_to_send_buffer(buffer, &msg);
        for (i=0; i<mavlink_msg_get_send_buffer_length(&msg); i++) {
            comm_send_ch(MAVLINK_COMM_0, buffer[i]);
        }
    mavlink_msg_cc_ai_diagnostic_decode(last_msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);
        
        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_ai_diagnostic_send(MAVLINK_COMM_1 , packet1.companion_timestamp_us , packet1.sequence , packet1.value , packet1.limit , packet1.detail_code , packet1.subsystem , packet1.severity , packet1.confidence_percent , packet1.schema_version );
    mavlink_msg_cc_ai_diagnostic_decode(last_msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

#ifdef MAVLINK_HAVE_GET_MESSAGE_INFO
    MAVLINK_ASSERT(mavlink_get_message_info_by_name("CC_AI_DIAGNOSTIC") != NULL);
    MAVLINK_ASSERT(mavlink_get_message_info_by_id(MAVLINK_MSG_ID_CC_AI_DIAGNOSTIC) != NULL);
#endif
}

static void mavlink_test_cc_mission_context(uint8_t system_id, uint8_t component_id, mavlink_message_t *last_msg)
{
#ifdef MAVLINK_STATUS_FLAG_OUT_MAVLINK1
    mavlink_status_t *status = mavlink_get_channel_status(MAVLINK_COMM_0);
        if ((status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1) && MAVLINK_MSG_ID_CC_MISSION_CONTEXT >= 256) {
            return;
        }
#endif
    mavlink_message_t msg;
        uint8_t buffer[MAVLINK_MAX_PACKET_LEN];
        uint16_t i;
    mavlink_cc_mission_context_t packet_in = {
        963497464,963497672,963497880,963498088,"QRSTUVWXYZABCDEFGHIJKLM",125
    };
    mavlink_cc_mission_context_t packet1, packet2;
        memset(&packet1, 0, sizeof(packet1));
        packet1.mission_id = packet_in.mission_id;
        packet1.cc_boot_id = packet_in.cc_boot_id;
        packet1.vehicle_id = packet_in.vehicle_id;
        packet1.dialect_hash = packet_in.dialect_hash;
        packet1.schema_version = packet_in.schema_version;
        
        mav_array_memcpy(packet1.sw_version, packet_in.sw_version, sizeof(char)*24);
        
#ifdef MAVLINK_STATUS_FLAG_OUT_MAVLINK1
        if (status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1) {
           // cope with extensions
           memset(MAVLINK_MSG_ID_CC_MISSION_CONTEXT_MIN_LEN + (char *)&packet1, 0, sizeof(packet1)-MAVLINK_MSG_ID_CC_MISSION_CONTEXT_MIN_LEN);
        }
#endif
        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_mission_context_encode(system_id, component_id, &msg, &packet1);
    mavlink_msg_cc_mission_context_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_mission_context_pack(system_id, component_id, &msg , packet1.mission_id , packet1.cc_boot_id , packet1.vehicle_id , packet1.dialect_hash , packet1.sw_version , packet1.schema_version );
    mavlink_msg_cc_mission_context_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_mission_context_pack_chan(system_id, component_id, MAVLINK_COMM_0, &msg , packet1.mission_id , packet1.cc_boot_id , packet1.vehicle_id , packet1.dialect_hash , packet1.sw_version , packet1.schema_version );
    mavlink_msg_cc_mission_context_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
        mavlink_msg_to_send_buffer(buffer, &msg);
        for (i=0; i<mavlink_msg_get_send_buffer_length(&msg); i++) {
            comm_send_ch(MAVLINK_COMM_0, buffer[i]);
        }
    mavlink_msg_cc_mission_context_decode(last_msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);
        
        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_mission_context_send(MAVLINK_COMM_1 , packet1.mission_id , packet1.cc_boot_id , packet1.vehicle_id , packet1.dialect_hash , packet1.sw_version , packet1.schema_version );
    mavlink_msg_cc_mission_context_decode(last_msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

#ifdef MAVLINK_HAVE_GET_MESSAGE_INFO
    MAVLINK_ASSERT(mavlink_get_message_info_by_name("CC_MISSION_CONTEXT") != NULL);
    MAVLINK_ASSERT(mavlink_get_message_info_by_id(MAVLINK_MSG_ID_CC_MISSION_CONTEXT) != NULL);
#endif
}

static void mavlink_test_cc_log_control(uint8_t system_id, uint8_t component_id, mavlink_message_t *last_msg)
{
#ifdef MAVLINK_STATUS_FLAG_OUT_MAVLINK1
    mavlink_status_t *status = mavlink_get_channel_status(MAVLINK_COMM_0);
        if ((status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1) && MAVLINK_MSG_ID_CC_LOG_CONTROL >= 256) {
            return;
        }
#endif
    mavlink_message_t msg;
        uint8_t buffer[MAVLINK_MAX_PACKET_LEN];
        uint16_t i;
    mavlink_cc_log_control_t packet_in = {
        93372036854775807ULL,963497880,41,108
    };
    mavlink_cc_log_control_t packet1, packet2;
        memset(&packet1, 0, sizeof(packet1));
        packet1.companion_timestamp_us = packet_in.companion_timestamp_us;
        packet1.sequence = packet_in.sequence;
        packet1.requested_profile = packet_in.requested_profile;
        packet1.schema_version = packet_in.schema_version;
        
        
#ifdef MAVLINK_STATUS_FLAG_OUT_MAVLINK1
        if (status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1) {
           // cope with extensions
           memset(MAVLINK_MSG_ID_CC_LOG_CONTROL_MIN_LEN + (char *)&packet1, 0, sizeof(packet1)-MAVLINK_MSG_ID_CC_LOG_CONTROL_MIN_LEN);
        }
#endif
        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_log_control_encode(system_id, component_id, &msg, &packet1);
    mavlink_msg_cc_log_control_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_log_control_pack(system_id, component_id, &msg , packet1.companion_timestamp_us , packet1.sequence , packet1.requested_profile , packet1.schema_version );
    mavlink_msg_cc_log_control_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_log_control_pack_chan(system_id, component_id, MAVLINK_COMM_0, &msg , packet1.companion_timestamp_us , packet1.sequence , packet1.requested_profile , packet1.schema_version );
    mavlink_msg_cc_log_control_decode(&msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

        memset(&packet2, 0, sizeof(packet2));
        mavlink_msg_to_send_buffer(buffer, &msg);
        for (i=0; i<mavlink_msg_get_send_buffer_length(&msg); i++) {
            comm_send_ch(MAVLINK_COMM_0, buffer[i]);
        }
    mavlink_msg_cc_log_control_decode(last_msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);
        
        memset(&packet2, 0, sizeof(packet2));
    mavlink_msg_cc_log_control_send(MAVLINK_COMM_1 , packet1.companion_timestamp_us , packet1.sequence , packet1.requested_profile , packet1.schema_version );
    mavlink_msg_cc_log_control_decode(last_msg, &packet2);
        MAVLINK_ASSERT(memcmp(&packet1, &packet2, sizeof(packet1)) == 0);

#ifdef MAVLINK_HAVE_GET_MESSAGE_INFO
    MAVLINK_ASSERT(mavlink_get_message_info_by_name("CC_LOG_CONTROL") != NULL);
    MAVLINK_ASSERT(mavlink_get_message_info_by_id(MAVLINK_MSG_ID_CC_LOG_CONTROL) != NULL);
#endif
}

static void mavlink_test_cc_dialect(uint8_t system_id, uint8_t component_id, mavlink_message_t *last_msg)
{
    mavlink_test_cc_telemetry_state(system_id, component_id, last_msg);
    mavlink_test_cc_telemetry_imu(system_id, component_id, last_msg);
    mavlink_test_cc_telemetry_power(system_id, component_id, last_msg);
    mavlink_test_cc_telemetry_gps(system_id, component_id, last_msg);
    mavlink_test_cc_telemetry_estimator(system_id, component_id, last_msg);
    mavlink_test_cc_telemetry_actuator(system_id, component_id, last_msg);
    mavlink_test_cc_event(system_id, component_id, last_msg);
    mavlink_test_cc_safety_status(system_id, component_id, last_msg);
    mavlink_test_cc_health_report(system_id, component_id, last_msg);
    mavlink_test_cc_ai_diagnostic(system_id, component_id, last_msg);
    mavlink_test_cc_mission_context(system_id, component_id, last_msg);
    mavlink_test_cc_log_control(system_id, component_id, last_msg);
}

#ifdef __cplusplus
}
#endif // __cplusplus
#endif // CC_DIALECT_TESTSUITE_H
