#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(binding = 0) uniform UniformBufferObject {
    mat4 model;
    mat4 view;
    mat4 proj;
} mvp;

layout(location = 0) in vec3 position;
layout(location = 2) in vec2 inTexCoord;
layout(location = 3) in vec2 inTexCoord2;

layout(location = 0) out vec2 fragTexCoord;
layout(location = 1) out vec2 fragTexCoord2;

mat4 clip = mat4(vec4(1.0, 0.0, 0.0, 0.0),
                 vec4(0.0, -1.0, 0.0, 0.0),
                 vec4(0.0, 0.0, 0.5, 0.5),
                 vec4(0.0, 0.0, 0, 1.0));

void main() {
    gl_Position = vec4(position, 1.0) * mvp.model * mvp.view * mvp.proj * clip;

    fragTexCoord = inTexCoord;
    fragTexCoord2 = inTexCoord2;
}
